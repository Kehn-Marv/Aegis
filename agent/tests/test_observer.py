"""Tests for observer trend computation."""

from __future__ import annotations

import time

from aegis_ops.models import GatewayStatus
from aegis_ops.observer import Observer, _GatewayHistory, _classify_trajectory


def _status(**kwargs) -> GatewayStatus:
    defaults = dict(
        uptime_secs=60,
        online=True,
        override_active=False,
        diagnostic_active=False,
        queue_depth=0,
        events_in=0,
        events_out=0,
        dedup_savings_pct=99.0,
        unique_signatures=0,
    )
    defaults.update(kwargs)
    return GatewayStatus(**defaults)


def test_trajectory_incident_likely() -> None:
    label = _classify_trajectory(
        events_in_per_min=1000.0,
        new_sigs_per_min=6.0,
        queue_delta=0,
        anomaly_rate=55.0,
        dedup_savings_pct=70.0,
    )
    assert label == "incident_likely"


def test_trajectory_degrading_queue() -> None:
    label = _classify_trajectory(
        events_in_per_min=100.0,
        new_sigs_per_min=1.0,
        queue_delta=150,
        anomaly_rate=5.0,
        dedup_savings_pct=95.0,
    )
    assert label == "degrading"


def test_trajectory_stable() -> None:
    label = _classify_trajectory(
        events_in_per_min=50.0,
        new_sigs_per_min=0.5,
        queue_delta=0,
        anomaly_rate=2.0,
        dedup_savings_pct=99.0,
    )
    assert label == "stable"


def test_compute_trends_with_history() -> None:
    hist = _GatewayHistory()
    now = time.time()
    prev = _status(events_in=100, unique_signatures=5, queue_depth=10)
    current = _status(events_in=400, unique_signatures=8, queue_depth=25)
    hist.push(now - 60.0, prev)
    hist.push(now, current)

    trends = Observer._compute_trends(hist, now, current, anomaly_count_5m=20)
    assert trends.events_in_per_min == 300.0
    assert trends.new_signatures_per_min == 3.0
    assert trends.queue_depth_delta == 15
    assert trends.anomaly_rate_per_min == 4.0
    assert trends.signature_velocity_rising is True
    assert trends.queue_growing is True
