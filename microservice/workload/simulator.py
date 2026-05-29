"""The self-driving simulation engine.

One coroutine drives the whole fleet. Most of the time it emits healthy
traffic; on its own schedule it injects an incident (cascade, crash-loop,
latency spike, or silence), lets Aegis react, then recovers. Nothing here
needs to be triggered by hand — start it and it runs.
"""

from __future__ import annotations

import asyncio
import logging
import random
import time
import uuid
from collections import deque
from dataclasses import dataclass, field

from .config import Settings
from .fleet import FLEET, FLEET_BY_NAME, ROUTINE_LOGS, SCENARIOS, SCENARIOS_BY_KEY, ServiceSpec
from .gateway_sink import GatewaySink
from .telemetry import Telemetry

log = logging.getLogger("workload.sim")

LEVEL_NAME = {logging.INFO: "INFO", logging.WARNING: "WARN", logging.ERROR: "ERROR", logging.DEBUG: "DEBUG"}


def _now_ts() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def _render(template: str) -> str:
    return template.format(
        ts=_now_ts(),
        rid=uuid.uuid4().hex[:12],
        uid=random.randint(1000, 99999),
        oid=uuid.uuid4().hex[:8],
        txn=uuid.uuid4().hex[:10],
        sku=f"SKU-{random.randint(100, 999)}",
        qty=random.randint(1, 5),
        amt=f"{random.uniform(5, 400):.2f}",
        lat=random.randint(8, 140),
        oct=random.randint(2, 254),
    )


@dataclass
class ServiceRuntime:
    spec: ServiceSpec
    status: str = "healthy"            # healthy | degraded | down | silent
    requests: int = 0
    errors: int = 0
    latencies: deque = field(default_factory=lambda: deque(maxlen=60))
    last_line: str = ""

    def p95(self) -> float:
        if not self.latencies:
            return 0.0
        ordered = sorted(self.latencies)
        idx = min(len(ordered) - 1, int(round(0.95 * (len(ordered) - 1))))
        return round(ordered[idx], 1)

    def error_rate(self) -> float:
        return round(self.errors / self.requests, 4) if self.requests else 0.0


class Simulator:
    def __init__(self, settings: Settings, telemetry: Telemetry, sink: GatewaySink) -> None:
        self.settings = settings
        self.telemetry = telemetry
        self.sink = sink
        self.runtimes: dict[str, ServiceRuntime] = {s.name: ServiceRuntime(s) for s in FLEET}

        self.started_at = time.time()
        self.autopilot = settings.autopilot
        self.active = None  # active scenario or None
        self._active_started = 0.0
        self._fired_steps: set[int] = set()
        self._next_incident_at = time.time() + random.uniform(20, 40)
        self.total_requests = 0
        self.total_errors = 0
        self.events: deque = deque(maxlen=40)
        self._inflight = 0
        self._task: asyncio.Task | None = None

        telemetry.set_gauge_source(self._gauge_values)

    # -- lifecycle -----------------------------------------------------------

    async def start(self) -> None:
        if self._task is None:
            self._task = asyncio.create_task(self._run(), name="simulator")

    async def stop(self) -> None:
        if self._task is not None:
            self._task.cancel()
            try:
                await self._task
            except asyncio.CancelledError:
                pass
            self._task = None

    # -- public control (driven by the API) ---------------------------------

    def trigger(self, key: str) -> bool:
        scenario = SCENARIOS_BY_KEY.get(key)
        if scenario is None or self.active is not None:
            return False
        self._begin_scenario(scenario)
        return True

    def set_autopilot(self, on: bool) -> None:
        self.autopilot = on
        if on:
            self._next_incident_at = time.time() + random.uniform(
                self.settings.incident_gap_min_secs, self.settings.incident_gap_max_secs
            )

    # -- the loop ------------------------------------------------------------

    async def _run(self) -> None:
        log.info("simulator online: %d services, autopilot=%s", len(FLEET), self.autopilot)
        self._note("system", "Workload online — emitting healthy traffic.")
        while True:
            tick_start = time.monotonic()
            now = time.time()
            self._maybe_schedule(now)
            self._fire_scenario_steps(now)
            self._emit_traffic()
            self._maybe_end_scenario(now)
            elapsed = time.monotonic() - tick_start
            await asyncio.sleep(max(0.05, self.settings.tick_secs - elapsed))

    def _maybe_schedule(self, now: float) -> None:
        if self.active is None and self.autopilot and now >= self._next_incident_at:
            self._begin_scenario(random.choice(SCENARIOS))

    def _begin_scenario(self, scenario) -> None:
        self.active = scenario
        self._active_started = time.time()
        self._fired_steps = set()
        for name, effect in scenario.degrade.items():
            rt = self.runtimes.get(name)
            if rt:
                rt.status = "silent" if effect.get("silent") else ("down" if scenario.severity == "red" else "degraded")
        self._note("incident", f"Injected '{scenario.title}': {scenario.summary}")
        log.warning("scenario started: %s", scenario.key)

    def _fire_scenario_steps(self, now: float) -> None:
        if self.active is None:
            return
        elapsed = now - self._active_started
        for idx, step in enumerate(self.active.steps):
            if idx in self._fired_steps or elapsed < step.at:
                continue
            self._fired_steps.add(idx)
            rt = self.runtimes.get(step.service)
            # Emit the step line several times so dedup has repeats to collapse.
            line = _render(step.template)
            for _ in range(6):
                self._ship(_render(step.template))
            self.telemetry.emit_log(step.level, step.service, line, {"scenario": self.active.key})
            if rt:
                rt.last_line = line
            with self.telemetry.request_span(step.service, "incident-step", {"scenario": self.active.key}) as span:
                if step.error:
                    self.telemetry.record_error(span, step.service, RuntimeError(line))

    def _maybe_end_scenario(self, now: float) -> None:
        if self.active is None:
            return
        if now - self._active_started >= self.active.duration_secs:
            recovered = self.active.title
            for name in self.active.degrade:
                rt = self.runtimes.get(name)
                if rt:
                    rt.status = "healthy"
            self._note("recovery", f"'{recovered}' cleared — fleet back to healthy.")
            log.info("scenario recovered: %s", self.active.key)
            self.active = None
            self._next_incident_at = now + random.uniform(
                self.settings.incident_gap_min_secs, self.settings.incident_gap_max_secs
            )

    # -- traffic generation --------------------------------------------------

    def _emit_traffic(self) -> None:
        degrade = self.active.degrade if self.active else {}
        count = max(1, int(self.settings.base_rps * random.uniform(0.8, 1.2)))
        weighted = [s for s in FLEET for _ in range(s.weight)]
        for _ in range(count):
            spec = random.choice(weighted)
            self._simulate_request(spec, degrade)

    def _simulate_request(self, spec: ServiceSpec, degrade: dict) -> None:
        effect = degrade.get(spec.name, {})
        if effect.get("silent"):
            return  # silent service emits nothing at all
        latency = spec.base_latency_ms * effect.get("latency_mult", 1.0) * random.uniform(0.7, 1.5)
        error_rate = effect.get("error_rate", spec.base_error_rate)
        ok = random.random() >= error_rate

        self._inflight += 1
        rt = self.runtimes[spec.name]
        with self.telemetry.request_span(spec.name, "handle", {"peer.deps": ",".join(spec.depends_on)}) as span:
            for dep in spec.depends_on:
                with self.telemetry.request_span(dep, "call", {"caller": spec.name}):
                    pass
            rt.requests += 1
            rt.latencies.append(latency)
            self.total_requests += 1
            if not ok:
                rt.errors += 1
                self.total_errors += 1
                self.telemetry.record_error(span, spec.name, RuntimeError(f"{spec.name} request failed"))
        self.telemetry.record_request(spec.name, "handle", latency, ok)
        self._inflight = max(0, self._inflight - 1)

        template = random.choice(ROUTINE_LOGS.get(spec.name, ["INFO  [{ts}] %s: ok" % spec.name]))
        if ok:
            # Sample routine INFO lines so the gateway sees steady, dedup-able
            # traffic (drives the "noise stopped" number) without flooding.
            if random.random() < 0.25:
                line = _render(template)
                self._ship(line)
                self.telemetry.emit_log(logging.INFO, spec.name, line)
                rt.last_line = line
        else:
            # Failures are always tracked in OTel (error tracking + metrics).
            # They are NOT streamed to the gateway: the causal story is driven
            # by the precisely-timed scenario step lines, so per-request errors
            # would only blur the temporal ordering (and seed false chains).
            err_line = _render(template).replace("INFO ", "ERROR").replace("200", "500")
            self.telemetry.emit_log(logging.ERROR, spec.name, err_line)
            if not self.active:
                rt.last_line = err_line

    def _ship(self, line: str) -> None:
        self.sink.emit(line)

    def _note(self, kind: str, message: str) -> None:
        self.events.appendleft({"ts": time.time(), "kind": kind, "message": message})

    # -- state for the gauges + API ------------------------------------------

    def _gauge_values(self) -> dict:
        load = 1.0 + (2.5 if self.active and self.active.severity == "red" else (1.2 if self.active else 0.0))
        return {
            "cpu": round(min(99.0, 22.0 * load + random.uniform(-4, 4)), 1),
            "memory": round(420.0 * (1 + 0.15 * load) + random.uniform(-20, 20), 1),
            "inflight": float(self._inflight),
        }

    def snapshot(self) -> dict:
        gauges = self._gauge_values()
        overall = "red" if (self.active and self.active.severity == "red") else (
            "orange" if self.active else "green"
        )
        return {
            "overall_state": overall,
            "autopilot": self.autopilot,
            "uptime_secs": int(time.time() - self.started_at),
            "total_requests": self.total_requests,
            "total_errors": self.total_errors,
            "error_rate": round(self.total_errors / self.total_requests, 4) if self.total_requests else 0.0,
            "inflight": self._inflight,
            "resources": gauges,
            "active_incident": (
                {
                    "key": self.active.key,
                    "title": self.active.title,
                    "summary": self.active.summary,
                    "severity": self.active.severity,
                    "elapsed_secs": round(time.time() - self._active_started, 1),
                    "duration_secs": self.active.duration_secs,
                }
                if self.active
                else None
            ),
            "next_incident_in_secs": (
                max(0, round(self._next_incident_at - time.time()))
                if (self.autopilot and self.active is None)
                else None
            ),
            "services": [
                {
                    "name": rt.spec.name,
                    "role": rt.spec.role,
                    "status": rt.status,
                    "requests": rt.requests,
                    "error_rate": rt.error_rate(),
                    "p95_ms": rt.p95(),
                    "depends_on": list(rt.spec.depends_on),
                    "last_line": rt.last_line,
                }
                for rt in self.runtimes.values()
            ],
            "scenarios": [
                {"key": s.key, "title": s.title, "summary": s.summary, "severity": s.severity}
                for s in SCENARIOS
            ],
            "events": list(self.events),
        }
