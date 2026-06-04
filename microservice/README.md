# Aegis Workload  -  self-driving telemetry microservice

This is the app that *produces* the telemetry Aegis protects. It simulates a
small e-commerce service fleet (`api-gateway`, `auth`, `checkout`,
`payment-api`, `orders`, …) and, on its own schedule, injects realistic
incidents  -  a payment cascade, an auth crash-loop, a latency spike, a silent
service  -  then recovers. You start it once; it decides what to do by itself.

It emits the full OpenTelemetry signal set and streams its raw log lines
straight into the Aegis gateway, so it replaces running `log_spammer.py`
patterns by hand.

| Signal | Where it comes from |
|--------|---------------------|
| **Logs** | structured records per request + every incident line |
| **Metrics** | request counter, latency histogram, CPU/memory/in-flight gauges |
| **Traces** | a span tree per request across the call graph |
| **Error tracking** | failed spans get `record_exception()` + `ERROR` status |
| **Performance monitoring** | the latency histogram (p50/p95/p99) + throughput |

## Run it

```powershell
cd microservice
py -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e .
python -m workload
```

Open **http://localhost:8080** for the live control room. That's the whole
setup  -  no flags, no manual traffic commands.

If the Aegis gateway is running (ingest on `tcp/5140`), the workload connects
automatically and you'll see incidents light up the Aegis decision card. If the
gateway isn't up yet, the workload keeps retrying in the background.

## Configuration (all optional, env-driven)

| Variable | Default | Purpose |
|----------|---------|---------|
| `WORKLOAD_PORT` | `8080` | Web UI / API port |
| `AEGIS_GATEWAY_HOST` / `AEGIS_GATEWAY_PORT` | `127.0.0.1` / `5140` | Gateway ingest target |
| `AEGIS_GATEWAY_ENABLED` | `true` | Stream raw logs to the gateway |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | *(unset)* | Point at an OTel Collector to ship to Splunk |
| `OTEL_EXPORTER_OTLP_PROTOCOL` | `http/protobuf` | OTLP transport |
| `SIM_AUTOPILOT` | `true` | Auto-inject incidents on a schedule |
| `SIM_BASE_RPS` | `24` | Baseline simulated requests/sec |

When `OTEL_EXPORTER_OTLP_ENDPOINT` is unset the app still *produces* all five
signals (visible at `GET /api/telemetry`)  -  it just doesn't try to export, so
there are no noisy connection errors.

## Shipping to Splunk (the modern path)

```text
workload ──OTLP──▶ OpenTelemetry Collector ──HEC──▶ Splunk
```

A ready collector config lives at [`otel-collector-config.yaml`](otel-collector-config.yaml).
Set your Splunk HEC token and run a collector with it, then start the workload
with `OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318`. Logs, metrics and
traces land in Splunk Observability Cloud / Splunk Enterprise.

## API

```text
GET  /api/state            full snapshot (fleet, signals, gateway, events)
GET  /api/telemetry        the five-signal summary + export status
GET  /api/health           liveness probe
POST /api/incident/{key}   inject a scenario now (cascade|crashloop|latency|silence)
POST /api/autopilot/{on|off}
```
