# Aegis

> **On-call shouldn't mean on-edge.** Aegis sits between your services
> and Splunk: it stops the alert storms, names the service that broke
> *first*, and remembers how every past incident was fixed so the next
> on-call already has the answer.

[![License: MIT](https://img.shields.io/badge/license-MIT-3DDC97?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-EB6228?style=flat-square&logo=rust)](Cargo.toml)
[![Python](https://img.shields.io/badge/python-3.11+-3776AB?style=flat-square&logo=python&logoColor=white)](sidecar/pyproject.toml)
[![Splunk MCP](https://img.shields.io/badge/MCP-server+client-7C5CFF?style=flat-square)](docs/mcp.md)
[![AppInspect](https://img.shields.io/badge/AppInspect-0%20failures-3DDC97?style=flat-square)](apps/aegis_ai/appinspect-report.json)

<p align="center">
  <strong>
    <a href="#getting-started">Getting Started</a>
    &nbsp;&nbsp;&bull;&nbsp;&nbsp;
    <a href="#mcp-control-plane">MCP</a>
    &nbsp;&nbsp;&bull;&nbsp;&nbsp;
    <a href="docs/architecture.md">Architecture</a>
    &nbsp;&nbsp;&bull;&nbsp;&nbsp;
    <a href="Troubleshooting.md">Troubleshooting</a>
  </strong>
</p>

Splunk Agentic Ops Hackathon 2026 · Observability track.

---

> [!WARNING]
> **Splunk Cloud access.** Some Splunk Cloud features (SLIM-backed Hosted
> Models, CDTSM forecasting) require provisioning that was not available on
> the 14-day trial. We reached out to Splunk support and the hackathon
> channel but did not get access in time. The integrations are fully built
> and wired; they activate the moment the environment is provisioned. We
> routed through AITK + local Ollama to keep the `| ai` pipeline live.
> Details in [`docs/splunk-blocker.md`](docs/splunk-blocker.md).

---

## What Aegis does

You've been here before. Something breaks in production, one service
starts throwing the same error thousands of times, and suddenly your
pager is going off, your ingest bill is climbing, and three other
services are failing too because they depend on the first one. Whose
fault is it? Was this the same thing that happened in March?

Aegis handles four things so you don't have to:

1. **Stops the noise.** One full copy of a repeating error goes through.
   The rest collapse into a clean count. Your ingest bill stays sane
   while you work the problem.

2. **Finds what broke first.** When everything starts failing at once,
   Aegis figures out which service went down *earliest* and tells you
   in one sentence, not 200 alerts.

3. **Remembers every fix.** After the fire is out, you write two
   sentences: what caused it, what fixed it. Six months later when the
   same pattern shows up, Aegis hands the next on-call the exact
   solution that worked last time. No digging through old docs at 2 AM.

4. **Keeps you in control.** No scary "Execute" button. You get one
   decision card with root cause, past fix, and a suggested next step.
   Three choices: `I'm on it`, `Show me more`, `This looks different`.
   Aegis never touches production.

Local, free, fast. SQLite for memory, Rust for the hot path, no
external services required.

```text
   workload app ──raw logs──▶  ┌─────────────┐
                               │    Aegis    │  ──processed events──▶  Splunk
   (OpenTelemetry) ──OTLP──▶   │   gateway   │
                               └──────┬──────┘
                                      │
                          decision card (UI · MCP · Splunk dashboard)
```

---

## Getting started

Two paths. Pick whichever fits.

| Path | What you get | Needs | Time |
|------|-------------|-------|------|
| **A** | Gateway + UI + self-driving workload. Full pipeline locally, no Splunk needed. | Docker (or Rust + Python + Node for source) | ~5 min |
| **B** | Path A plus HEC ingest, Splunk dashboard, AI sidecar, autonomous agent, multi-edge. | + Splunk Enterprise, Ollama | ~45 min |

From-source builds need **Rust 1.80+**, **Python 3.11+**, **Node 20+**,
and (on Windows) MSVC Build Tools.

---

### Path A: quick start

**Option 1: Docker (fastest)**

```powershell
docker compose up --build
```

* **Aegis control panel** → http://localhost:7321
* **Workload control room** → http://localhost:8080

That's it. The workload generates traffic and injects incidents on its
own (cascade, crash-loop, latency spike, silence). Open the control
panel and watch a decision card form, name the root cause, and recall
past fixes. No Splunk required.

To ship the workload's OpenTelemetry data to Splunk as well, add the
collector profile:

```powershell
$env:SPLUNK_HEC_TOKEN = "<your-hec-token>"
$env:OTEL_EXPORTER_OTLP_ENDPOINT = "http://otel-collector:4318"
docker compose --profile splunk up --build
```

**Option 2: from source**

```powershell
git clone https://github.com/<your-handle>/aegis
cd aegis
cargo test --workspace                                   # 52 tests
cargo build --bin aegis-daemon
.\target\debug\aegis-daemon.exe --config configs\aegis.demo.toml
```

In a second terminal, start the self-driving workload:

```powershell
cd microservice
py -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e .
python -m workload
```

Or fire a single pattern by hand:

```powershell
python demo\log_spammer.py --target tcp://127.0.0.1:5140 --pattern cascade
```

Other manual patterns: `crashloop`, `routine`, `silence`.

**Build the UI** (first time only):

```powershell
cd ui
npm install && npm run build    # daemon serves the UI at http://localhost:7321
```

For hot-reload during development: `npm run dev` → http://localhost:5173

**Try the memory loop:**

1. Run a `cascade` pattern. The decision card goes red:

```json
{
  "state": "red",
  "root_cause_service": "payment-api",
  "headline": "payment-api broke first. checkout followed 4s later. orders followed 8s later.",
  "suggested_next_step": "Check payment-api health and recent deploys..."
}
```

2. Resolve the incident: write what caused it and what fixed it.
3. Run the cascade again. The new card now carries the past fix:

```json
"suggested_next_step": "This looks 100% similar to a past incident (fixed in 2 min). Last time the cause was: \"DB pool exhausted under retry storm.\" The fix was: \"Increased pool to 32, retry interval to 30s.\" Start by verifying that."
```

That's the core of Aegis. The institutional memory of every previous
on-call, at the fingertips of the new one.

---

### Path B: full Splunk integration

Complete Path A first, then add Splunk and the agent.

**B1. Splunk + HEC**

1. Install Splunk Enterprise (free trial). Open http://localhost:8000.
2. Create an index named `aegis`.
3. Create an HEC token: name `aegis`, index `aegis`, sourcetype auto.
4. Copy `configs\aegis.example.toml` → `configs\aegis.toml`, paste the token.
5. Verify: `cargo run --bin aegis-daemon -- --check-hec`

If you hit a 401 or 404, check [`Troubleshooting.md`](Troubleshooting.md).

**B2. AI sidecar**

```powershell
cd sidecar
py -m venv .venv && .\.venv\Scripts\Activate.ps1
pip install -e . --extra-index-url https://download.pytorch.org/whl/cpu
python -m aegis_sidecar.server    # http://127.0.0.1:8765
```

**B3. Run the pipeline** (3 terminals: sidecar, daemon, spammer)

```powershell
cargo run --release --bin aegis-daemon
python demo\log_spammer.py --target tcp://127.0.0.1:5140 --pattern cascade
```

Check Splunk (Search & Reporting, last 15 minutes):

```spl
index=aegis sourcetype=aegis:decision | head 5
index=aegis sourcetype=aegis:causal   | head 5
index=aegis sourcetype=aegis:incident | head 5
```

When you are done with B3, **stop the daemon** (Ctrl+C). It holds
ports 5140 and 7321 that Path B6 needs. If you later see `bind ...
already in use`, see [Troubleshooting](Troubleshooting.md#port-already-in-use-windows).

**B4. Dashboard**

Search & Reporting → Dashboards → Create New → name `Aegis` → Dashboard
Studio → Absolute layout → paste [`dashboards/aegis.json`](dashboards/aegis.json) → Save.

The CDTSM forecast panels at the bottom need Splunk AI Toolkit +
Splunk Cloud with SLIM provisioning. See
[`docs/splunk-blocker.md`](docs/splunk-blocker.md). The other panels
work on local Enterprise.

**B5. Splunk app** (optional)

```powershell
Copy-Item -Recurse apps\aegis_ai "$env:SPLUNK_HOME\etc\apps\aegis_ai"
& "$env:SPLUNK_HOME\bin\splunk" restart
```

AppInspect: **0 failures** ([report](apps/aegis_ai/appinspect-report.json)).
See [`apps/aegis_ai/README.md`](apps/aegis_ai/README.md) for LLM config.

**B6. AegisOps agent + multi-edge** (optional)

```powershell
ollama pull qwen2.5:3b

# Two regional gateways:
Copy-Item configs\aegis.us-east.example.toml configs\aegis-us-east.toml
Copy-Item configs\aegis.eu-west.example.toml configs\aegis-eu-west.toml
# Edit both: paste your HEC token

# Terminal A: cargo run --release --bin aegis-daemon -- --config configs\aegis-us-east.toml
# Terminal B: cargo run --release --bin aegis-daemon -- --config configs\aegis-eu-west.toml

# Agent:
cd agent
py -m venv .venv && .\.venv\Scripts\Activate.ps1
pip install -e .
Copy-Item configs\aegis-ops.example.toml configs\aegis-ops.toml
# Edit: set model under [llm.ollama], paste tokens under [splunk] & [audit]

aegis-ops --config configs\aegis-ops.toml --once -v   # smoke test
aegis-ops --config configs\aegis-ops.toml -v            # continuous
```

Each tick, the agent reads the gateway's own decision card, forwards it
(with similar past fixes) into the LLM prompt, and optionally actuates
a bounded diagnostic command. Every action is audited to Splunk HEC
(`sourcetype=aegis:agent`). See [`agent/README.md`](agent/README.md)
for the transport matrix and policy modes.

---

## MCP control plane

Aegis is on **both sides** of MCP:

* **Aegis as MCP server** at `http://127.0.0.1:7321/mcp`. Tools:

  | Tool                 | What it does                                    |
  |----------------------|-------------------------------------------------|
  | `status`             | Live gateway snapshot                           |
  | `latest_decision`    | Current decision card (null if green)            |
  | `recent_incidents`   | Top-N fingerprints from memory                   |
  | `resolve_incident`   | Attach a cause + fix to a past incident          |
  | `acknowledge`        | Mark current decision as "I'm on it"             |
  | `diagnostic` / `override` / `reset` / `replay_raw` | Bounded-window observability tools |

* **AegisOps Agent as MCP client** of the official Splunk MCP Server.
  Every SPL call goes through `tools/call`, fully auditable in
  `index=_internal sourcetype=mcpjson`.

Wire Aegis into Cursor or Claude Desktop in two lines. See
[`docs/mcp.md`](docs/mcp.md).

---

## Troubleshooting

Common issues and fixes live in [`Troubleshooting.md`](Troubleshooting.md).
Quick hits:

* **`bind tcp listener ... already in use`** : another daemon is
  running. `Get-Process aegis-daemon | Stop-Process -Force`.
* **`HEC rejected events: 401`** : bad or disabled token. Re-issue in
  Splunk Web and update `configs/aegis.toml`.
* **Dashboard CDTSM panels show 404** : expected on local Enterprise;
  the rest of the dashboard works. See
  [`docs/splunk-blocker.md`](docs/splunk-blocker.md).
* **Decision card never goes red** : make sure you're running the
  `cascade` pattern and that it fits inside `[causal].window_secs`
  (30s default in demo configs).

---

## License

[MIT](LICENSE).
