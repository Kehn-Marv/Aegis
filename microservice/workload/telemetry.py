"""OpenTelemetry wiring for the workload — the five signals in one place.

| Signal              | How it shows up here                                          |
|---------------------|---------------------------------------------------------------|
| Logs                | `emit_log()` -> OTel LogRecord -> OTLP                          |
| Metrics             | request counter, latency histogram, resource gauges            |
| Traces              | `request_span()` -> per-request span tree across the fleet     |
| Error tracking      | span status = ERROR + `record_exception()` + error counter     |
| Performance monitor | latency histogram (p50/p95/p99) + throughput counter           |

Exporters are only attached when `OTEL_EXPORTER_OTLP_ENDPOINT` is set, so
the app runs cleanly with or without a collector in front of it.
"""

from __future__ import annotations

import logging
from contextlib import contextmanager
from typing import Callable, Iterator

from opentelemetry import metrics, trace
from opentelemetry.sdk.resources import Resource
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor, ConsoleSpanExporter
from opentelemetry.sdk.metrics import MeterProvider
from opentelemetry.sdk.metrics.export import (
    ConsoleMetricExporter,
    PeriodicExportingMetricReader,
)
from opentelemetry._logs import set_logger_provider
from opentelemetry.sdk._logs import LoggerProvider, LoggingHandler
from opentelemetry.sdk._logs.export import (
    BatchLogRecordProcessor,
    ConsoleLogExporter,
)
from opentelemetry.trace import Status, StatusCode

from .config import Settings

log = logging.getLogger("workload.telemetry")

GaugeSource = Callable[[], dict[str, float]]


class Telemetry:
    """Owns the OTel providers + instruments and exposes ergonomic helpers."""

    def __init__(self, settings: Settings) -> None:
        self.settings = settings
        self.exporting = settings.otlp_enabled or settings.console_telemetry
        self._gauge_source: GaugeSource | None = None

        # Counters surfaced on the dashboard so the five signals are visible
        # even without a collector wired up.
        self.counts = {
            "logs": 0,
            "traces": 0,
            "metrics": 0,
            "errors": 0,
        }

        resource = Resource.create(
            {
                "service.name": settings.service_name,
                "service.version": settings.version,
                "deployment.environment": settings.environment,
            }
        )

        self._init_traces(resource)
        self._init_metrics(resource)
        self._init_logs(resource)

        self.tracer = trace.get_tracer("workload.fleet", settings.version)
        self.meter = metrics.get_meter("workload.fleet", settings.version)

        self.request_counter = self.meter.create_counter(
            "workload.requests", unit="1", description="Simulated requests handled by the fleet."
        )
        self.error_counter = self.meter.create_counter(
            "workload.errors", unit="1", description="Simulated requests that failed."
        )
        self.latency_hist = self.meter.create_histogram(
            "workload.request.duration", unit="ms", description="Per-request latency."
        )
        self.meter.create_observable_gauge(
            "workload.cpu.utilization", callbacks=[self._observe_cpu], unit="%"
        )
        self.meter.create_observable_gauge(
            "workload.memory.usage", callbacks=[self._observe_mem], unit="MiBy"
        )
        self.meter.create_observable_gauge(
            "workload.active.requests", callbacks=[self._observe_inflight], unit="1"
        )

        # A dedicated logger whose records become OTel logs. propagate=False
        # keeps the high-volume simulated logs off the process stdout.
        self.fleet_logger = logging.getLogger("workload.fleet.signals")
        self.fleet_logger.setLevel(logging.INFO)
        self.fleet_logger.propagate = False
        if self._log_handler is not None:
            self.fleet_logger.addHandler(self._log_handler)

    # -- provider setup ------------------------------------------------------

    def _init_traces(self, resource: Resource) -> None:
        provider = TracerProvider(resource=resource)
        if self.settings.otlp_enabled:
            from opentelemetry.exporter.otlp.proto.http.trace_exporter import OTLPSpanExporter

            provider.add_span_processor(BatchSpanProcessor(OTLPSpanExporter()))
        if self.settings.console_telemetry:
            provider.add_span_processor(BatchSpanProcessor(ConsoleSpanExporter()))
        trace.set_tracer_provider(provider)
        self._tracer_provider = provider

    def _init_metrics(self, resource: Resource) -> None:
        readers = []
        if self.settings.otlp_enabled:
            from opentelemetry.exporter.otlp.proto.http.metric_exporter import OTLPMetricExporter

            readers.append(
                PeriodicExportingMetricReader(
                    OTLPMetricExporter(),
                    export_interval_millis=self.settings.metric_export_secs * 1000,
                )
            )
        if self.settings.console_telemetry:
            readers.append(
                PeriodicExportingMetricReader(
                    ConsoleMetricExporter(),
                    export_interval_millis=self.settings.metric_export_secs * 1000,
                )
            )
        provider = MeterProvider(resource=resource, metric_readers=readers)
        metrics.set_meter_provider(provider)
        self._meter_provider = provider

    def _init_logs(self, resource: Resource) -> None:
        self._log_handler = None
        provider = LoggerProvider(resource=resource)
        attached = False
        if self.settings.otlp_enabled:
            from opentelemetry.exporter.otlp.proto.http._log_exporter import OTLPLogExporter

            provider.add_log_record_processor(BatchLogRecordProcessor(OTLPLogExporter()))
            attached = True
        if self.settings.console_telemetry:
            provider.add_log_record_processor(BatchLogRecordProcessor(ConsoleLogExporter()))
            attached = True
        set_logger_provider(provider)
        self._logger_provider = provider
        if attached:
            self._log_handler = LoggingHandler(level=logging.INFO, logger_provider=provider)

    # -- gauge callbacks -----------------------------------------------------

    def set_gauge_source(self, source: GaugeSource) -> None:
        self._gauge_source = source

    def _gauge(self, key: str) -> list:
        from opentelemetry.metrics import Observation

        if self._gauge_source is None:
            return []
        value = self._gauge_source().get(key)
        return [Observation(value)] if value is not None else []

    def _observe_cpu(self, _options) -> list:
        return self._gauge("cpu")

    def _observe_mem(self, _options) -> list:
        return self._gauge("memory")

    def _observe_inflight(self, _options) -> list:
        return self._gauge("inflight")

    # -- ergonomic helpers used by the simulator -----------------------------

    @contextmanager
    def request_span(self, service: str, operation: str, attrs: dict | None = None) -> Iterator:
        with self.tracer.start_as_current_span(f"{service} {operation}") as span:
            span.set_attribute("service.name", service)
            span.set_attribute("operation", operation)
            for key, value in (attrs or {}).items():
                span.set_attribute(key, value)
            self.counts["traces"] += 1
            yield span

    def record_request(self, service: str, operation: str, duration_ms: float, ok: bool) -> None:
        labels = {"service": service, "operation": operation, "status": "ok" if ok else "error"}
        self.request_counter.add(1, labels)
        self.latency_hist.record(duration_ms, labels)
        self.counts["metrics"] += 1
        if not ok:
            self.error_counter.add(1, {"service": service, "operation": operation})

    def record_error(self, span, service: str, exc: Exception) -> None:
        span.record_exception(exc)
        span.set_status(Status(StatusCode.ERROR, str(exc)))
        self.counts["errors"] += 1

    def emit_log(self, level: int, service: str, message: str, attrs: dict | None = None) -> None:
        extra = {"service.name": service, **(attrs or {})}
        self.fleet_logger.log(level, message, extra={"otel_attributes": extra})
        self.counts["logs"] += 1

    def telemetry_status(self) -> dict:
        return {
            "exporting": self.exporting,
            "otlp_enabled": self.settings.otlp_enabled,
            "otlp_endpoint": self.settings.otlp_endpoint or None,
            "otlp_protocol": self.settings.otlp_protocol,
            "console_export": self.settings.console_telemetry,
            "signals": dict(self.counts),
        }

    def shutdown(self) -> None:
        for provider in (self._tracer_provider, self._meter_provider, self._logger_provider):
            try:
                provider.shutdown()
            except Exception:  # noqa: BLE001 - best-effort flush on exit
                pass
