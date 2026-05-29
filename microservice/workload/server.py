"""FastAPI app: serves the control-room UI and the JSON API, and owns the
lifecycle of the telemetry stack, the gateway sink, and the simulator.
"""

from __future__ import annotations

import logging
from pathlib import Path

from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.responses import JSONResponse
from fastapi.staticfiles import StaticFiles

from .config import settings
from .gateway_sink import GatewaySink
from .simulator import Simulator
from .telemetry import Telemetry

logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)-5s %(name)s: %(message)s")
log = logging.getLogger("workload")

STATIC_DIR = Path(__file__).parent / "static"


@asynccontextmanager
async def lifespan(app: FastAPI):
    log.info("starting aegis-workload v%s (env=%s)", settings.version, settings.environment)
    telemetry = Telemetry(settings)
    sink = GatewaySink(settings.gateway_host, settings.gateway_port, enabled=settings.gateway_enabled)
    simulator = Simulator(settings, telemetry, sink)

    app.state.telemetry = telemetry
    app.state.sink = sink
    app.state.simulator = simulator

    await sink.start()
    await simulator.start()
    if settings.otlp_enabled:
        log.info("exporting OTLP telemetry to %s", settings.otlp_endpoint)
    else:
        log.info("no OTEL_EXPORTER_OTLP_ENDPOINT set — telemetry is produced but not exported")
    try:
        yield
    finally:
        await simulator.stop()
        await sink.stop()
        telemetry.shutdown()
        log.info("aegis-workload stopped")


app = FastAPI(title="Aegis Workload", version=settings.version, lifespan=lifespan)

# Best-effort HTTP server auto-instrumentation. Optional: a version skew in
# the instrumentation package must never stop the app from booting.
try:  # pragma: no cover - environment dependent
    from opentelemetry.instrumentation.fastapi import FastAPIInstrumentor

    FastAPIInstrumentor.instrument_app(app)
except Exception as exc:  # noqa: BLE001
    log.info("fastapi auto-instrumentation unavailable (%s); manual spans still active", exc)


@app.get("/api/health")
async def health() -> dict:
    return {"status": "ok", "service": settings.service_name, "version": settings.version}


@app.get("/api/state")
async def state() -> JSONResponse:
    sim: Simulator = app.state.simulator
    snap = sim.snapshot()
    snap["gateway"] = app.state.sink.stats
    snap["telemetry"] = app.state.telemetry.telemetry_status()
    return JSONResponse(snap)


@app.get("/api/telemetry")
async def telemetry() -> dict:
    return app.state.telemetry.telemetry_status()


@app.post("/api/incident/{key}")
async def trigger_incident(key: str) -> dict:
    ok = app.state.simulator.trigger(key)
    return {"ok": ok, "scenario": key}


@app.post("/api/autopilot/{state}")
async def autopilot(state: str) -> dict:
    on = state.lower() in {"on", "true", "1"}
    app.state.simulator.set_autopilot(on)
    return {"ok": True, "autopilot": on}


if STATIC_DIR.is_dir():
    app.mount("/", StaticFiles(directory=str(STATIC_DIR), html=True), name="static")
