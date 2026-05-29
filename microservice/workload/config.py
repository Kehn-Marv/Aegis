"""Runtime configuration, entirely env-driven (12-factor).

Every value has a sane default so the app runs with zero configuration:
just `python -m workload` (or start the container) and it works. Point it
at an OpenTelemetry Collector by setting `OTEL_EXPORTER_OTLP_ENDPOINT` and
the same telemetry starts flowing to Splunk.
"""

from __future__ import annotations

import os
from dataclasses import dataclass, field


def _env_bool(name: str, default: bool) -> bool:
    raw = os.getenv(name)
    if raw is None:
        return default
    return raw.strip().lower() in {"1", "true", "yes", "on"}


def _env_int(name: str, default: int) -> int:
    try:
        return int(os.getenv(name, "").strip())
    except (TypeError, ValueError):
        return default


def _env_float(name: str, default: float) -> float:
    try:
        return float(os.getenv(name, "").strip())
    except (TypeError, ValueError):
        return default


@dataclass(frozen=True)
class Settings:
    # ---- Web app -----------------------------------------------------------
    host: str = field(default_factory=lambda: os.getenv("WORKLOAD_HOST", "0.0.0.0"))
    port: int = field(default_factory=lambda: _env_int("WORKLOAD_PORT", 8080))

    # ---- Identity (also used as OTel resource attributes) ------------------
    service_name: str = field(default_factory=lambda: os.getenv("OTEL_SERVICE_NAME", "aegis-workload"))
    environment: str = field(default_factory=lambda: os.getenv("DEPLOY_ENV", "demo"))
    version: str = field(default_factory=lambda: os.getenv("WORKLOAD_VERSION", "0.1.0"))

    # ---- Aegis gateway egress (raw log lines over TCP) ---------------------
    gateway_enabled: bool = field(default_factory=lambda: _env_bool("AEGIS_GATEWAY_ENABLED", True))
    gateway_host: str = field(default_factory=lambda: os.getenv("AEGIS_GATEWAY_HOST", "127.0.0.1"))
    gateway_port: int = field(default_factory=lambda: _env_int("AEGIS_GATEWAY_PORT", 5140))

    # ---- OpenTelemetry -----------------------------------------------------
    # If an OTLP endpoint is configured we export logs/metrics/traces to it.
    # Otherwise the app still produces telemetry (visible in /api/telemetry)
    # but does not try to ship it, so there are no noisy connection errors.
    otlp_endpoint: str = field(default_factory=lambda: os.getenv("OTEL_EXPORTER_OTLP_ENDPOINT", "").strip())
    otlp_protocol: str = field(
        default_factory=lambda: os.getenv("OTEL_EXPORTER_OTLP_PROTOCOL", "http/protobuf").strip()
    )
    console_telemetry: bool = field(default_factory=lambda: _env_bool("OTEL_CONSOLE_EXPORT", False))
    metric_export_secs: int = field(default_factory=lambda: _env_int("OTEL_METRIC_EXPORT_SECS", 10))

    # ---- Simulation engine -------------------------------------------------
    tick_secs: float = field(default_factory=lambda: _env_float("SIM_TICK_SECS", 1.0))
    base_rps: int = field(default_factory=lambda: _env_int("SIM_BASE_RPS", 24))
    # Seconds of healthy traffic between auto-injected incidents (jittered).
    incident_gap_min_secs: int = field(default_factory=lambda: _env_int("SIM_INCIDENT_GAP_MIN", 75))
    incident_gap_max_secs: int = field(default_factory=lambda: _env_int("SIM_INCIDENT_GAP_MAX", 150))
    autopilot: bool = field(default_factory=lambda: _env_bool("SIM_AUTOPILOT", True))

    @property
    def gateway_addr(self) -> tuple[str, int]:
        return (self.gateway_host, self.gateway_port)

    @property
    def otlp_enabled(self) -> bool:
        return bool(self.otlp_endpoint)


settings = Settings()
