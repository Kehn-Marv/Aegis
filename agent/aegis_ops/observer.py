"""Observer: builds a per-gateway `Observation` each tick.

Two data sources:

* **Live gateway state** via the Aegis REST API (always available).
* **Splunk-derived signals** via SPL queries against `index=aegis`
  (optional — skipped gracefully when Splunk credentials are absent).

The observer keeps a small in-memory history per gateway so it can
compute rolling trends and a `trajectory` label between ticks.
"""

from __future__ import annotations

import logging
import time
from collections import deque
from dataclasses import dataclass, field

from .gateway_client import GatewayClient
from .models import GatewayStatus, Observation, TopSignature, TrajectoryLabel, Trends
from .splunk_client import SplunkClient

log = logging.getLogger("aegis_ops.observer")

TOP_N_SIGNATURES = 5


@dataclass
class _GatewayHistory:
    """Bounded sliding window for one gateway, used to compute trends."""

    samples: deque = field(default_factory=lambda: deque(maxlen=10))

    def push(self, ts: float, status: GatewayStatus) -> None:
        self.samples.append((ts, status))

    def trend_window(self, now: float) -> tuple[float | None, GatewayStatus | None]:
        if not self.samples:
            return (None, None)
        target = now - 60.0
        prev_ts, prev_status = self.samples[0]
        for ts, status in self.samples:
            if ts <= target:
                prev_ts, prev_status = ts, status
            else:
                break
        return (prev_ts, prev_status)


class Observer:
    def __init__(
        self,
        splunk: SplunkClient | None,
        earliest: str = "-5m",
        latest: str = "now",
    ):
        self.splunk = splunk
        self.earliest = earliest
        self.latest = latest
        self._history: dict[str, _GatewayHistory] = {}

    async def observe(self, gateway_name: str, gateway: GatewayClient) -> Observation:
        status = await gateway.status()
        hist = self._history.setdefault(gateway_name, _GatewayHistory())
        now = time.time()
        hist.push(now, status)

        notes: list[str] = []
        top_sigs: list[TopSignature] = []
        anomaly_count = 0
        routine_count = 0
        unknown_count = 0

        if self.splunk is None:
            notes.append("splunk_disabled: running without SPL observations")
        else:
            try:
                top_sigs = await self._top_signatures(gateway_name)
            except Exception as exc:
                notes.append(f"top_signatures_unavailable: {exc}")
            try:
                anomaly_count, routine_count, unknown_count = await self._classifier_counts(
                    gateway_name
                )
            except Exception as exc:
                notes.append(f"classifier_counts_unavailable: {exc}")

        trends = self._compute_trends(hist, now, status, anomaly_count)

        return Observation(
            gateway=gateway_name,
            gateway_url=gateway.base,
            status=status,
            top_signatures=top_sigs,
            anomaly_count_5m=anomaly_count,
            routine_count_5m=routine_count,
            unknown_count_5m=unknown_count,
            trends=trends,
            notes=notes,
        )

    async def _top_signatures(self, gateway_name: str) -> list[TopSignature]:
        assert self.splunk is not None
        spl = (
            f'search index=aegis sourcetype=aegis:metric host={gateway_name} '
            f'| stats sum(count) AS suppressed, values(sample) AS sample, '
            f'  values("classification.label") AS label, '
            f'  values("classification.confidence") AS conf '
            f'  by signature '
            f'| sort -suppressed '
            f'| head {TOP_N_SIGNATURES}'
        )
        rows = await self.splunk.oneshot(spl, earliest=self.earliest, latest=self.latest)
        out: list[TopSignature] = []
        for row in rows:
            try:
                count_val = int(float(row.get("suppressed", 0)))
            except (TypeError, ValueError):
                count_val = 0
            sample = row.get("sample")
            if isinstance(sample, list):
                sample = sample[0] if sample else None
            label = row.get("label")
            if isinstance(label, list):
                label = label[0] if label else None
            conf = row.get("conf")
            if isinstance(conf, list):
                conf = conf[0] if conf else None
            try:
                conf_val = float(conf) if conf is not None else None
            except (TypeError, ValueError):
                conf_val = None
            out.append(
                TopSignature(
                    signature=str(row.get("signature", "")),
                    count=count_val,
                    sample=str(sample) if sample else None,
                    classification_label=str(label) if label else None,
                    classification_confidence=conf_val,
                )
            )
        return out

    async def _classifier_counts(self, gateway_name: str) -> tuple[int, int, int]:
        assert self.splunk is not None
        spl = (
            f'search index=aegis sourcetype=aegis:metric host={gateway_name} '
            f'"classification.label"=* '
            f'| stats sum(count) AS n by "classification.label"'
        )
        rows = await self.splunk.oneshot(spl, earliest=self.earliest, latest=self.latest)
        anomaly = routine = unknown = 0
        for row in rows:
            label = str(row.get("classification.label", "")).lower()
            try:
                n = int(float(row.get("n", 0)))
            except (TypeError, ValueError):
                n = 0
            if label == "anomaly":
                anomaly = n
            elif label == "routine":
                routine = n
            elif label == "unknown":
                unknown = n
        return anomaly, routine, unknown

    @staticmethod
    def _compute_trends(
        hist: _GatewayHistory,
        now: float,
        current: GatewayStatus,
        anomaly_count_5m: int,
    ) -> Trends:
        prev_ts, prev_status = hist.trend_window(now)
        anomaly_rate = anomaly_count_5m / 5.0
        if prev_status is None or prev_ts is None or prev_ts >= now:
            return Trends(anomaly_rate_per_min=anomaly_rate)

        dt_min = max((now - prev_ts) / 60.0, 1 / 60.0)
        events_in_per_min = (current.events_in - prev_status.events_in) / dt_min
        new_sigs_per_min = max(
            0.0,
            (current.unique_signatures - prev_status.unique_signatures) / dt_min,
        )
        queue_delta = current.queue_depth - prev_status.queue_depth

        sig_rising = new_sigs_per_min >= 3.0
        queue_growing = queue_delta > 0
        trajectory = _classify_trajectory(
            events_in_per_min=events_in_per_min,
            new_sigs_per_min=new_sigs_per_min,
            queue_delta=queue_delta,
            anomaly_rate=anomaly_rate,
            dedup_savings_pct=current.dedup_savings_pct,
        )

        return Trends(
            events_in_per_min=events_in_per_min,
            new_signatures_per_min=new_sigs_per_min,
            queue_depth_delta=queue_delta,
            anomaly_rate_per_min=anomaly_rate,
            signature_velocity_rising=sig_rising,
            queue_growing=queue_growing,
            trajectory=trajectory,
        )


def _classify_trajectory(
    *,
    events_in_per_min: float,
    new_sigs_per_min: float,
    queue_delta: int,
    anomaly_rate: float,
    dedup_savings_pct: float,
) -> TrajectoryLabel:
    if anomaly_rate >= 50.0 or (new_sigs_per_min >= 5.0 and anomaly_rate >= 10.0):
        return "incident_likely"
    if queue_delta > 100 or (queue_delta > 0 and events_in_per_min > 500.0):
        return "degrading"
    if new_sigs_per_min >= 3.0 and dedup_savings_pct < 80.0:
        return "degrading"
    return "stable"
