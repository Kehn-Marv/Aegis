# Aegis

> **On-call shouldn't mean on-edge.** Aegis sits between your services
> and Splunk: it stops the alert storms, names the service that broke
> *first*, and remembers how every past incident was fixed — so when
> production breaks again, you already have the answer.

[![License: MIT](https://img.shields.io/badge/license-MIT-3DDC97?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-EB6228?style=flat-square&logo=rust)](Cargo.toml)
[![Python](https://img.shields.io/badge/python-3.11+-3776AB?style=flat-square&logo=python&logoColor=white)](sidecar/pyproject.toml)
[![Splunk MCP](https://img.shields.io/badge/MCP-server+client-7C5CFF?style=flat-square)](docs/mcp.md)
[![AppInspect](https://img.shields.io/badge/AppInspect-0%20failures-3DDC97?style=flat-square)](apps/aegis_ai/appinspect-report.json)

Splunk Agentic Ops Hackathon 2026 · Observability track.

---

## What Aegis does

A service crash-loops at 3 AM and fires the same error 10,000 times.
Here's what happens:

1. **Stops the noise.** Aegis sends one full copy of a repeating error
   and collapses the rest into a clean count. Your ingest bill stays
   sane while you work the problem.

2. **Finds what broke first.** When everything starts failing at once,
   Aegis figures out which service went down *earliest* and tells you in
   one sentence — not 200 alerts.

3. **Remembers every fix.** After the fire is out, you write two
   sentences: what caused it, what fixed it. Six months later when the
   same pattern shows up, Aegis hands the next on-call the exact
   solution that worked last time. No digging through old docs at 2 AM.

4. **Keeps you in control.** No scary "Execute" button. You get one
   decision card — root cause, past fix, suggested next step — and three
   choices: `I'm on it`, `Show me more`, `This looks different`. Aegis
   never touches production.

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
| **A — Quick start** | Gateway + UI + self-driving workload. See the full pipeline locally, no Splunk needed. | Docker (or Rust + Python + Node for source) | ~5 min |
| **B — Full Splunk integration** | Path A plus HEC ingest, Splunk dashboard, AI sidecar, autonomous agent, multi-edge. | + Splunk Enterprise, Ollama | ~45 min |

From-source builds need **Rust 1.80+**, **Python 3.11+**, **Node 20+**,
and (on Windows) MSVC Build Tools.

---

### Path A — Quick start

**Option 1: Docker (fastest)**

```powershell
docker compose up --build
```

* **Aegis control panel** → http://localhost:7321
* **Workload control room** → http://localhost:8080

That's it. The workload generates traffic and injects incidents on its
own — open the control panel and watch a decision card form, name the
root cause, and recall past fixes. No Splunk required.

**Option 2: From source**

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

**Build the UI** (first time only):

```powershell
cd ui
npm install && npm run build    # daemon serves the UI at http://localhost:7321
```

For hot-reload during development: `npm run dev` → http://localhost:5173

**Try the memory loop:**

1. Run a `cascade` pattern. The decision card goes red with root cause.
2. Resolve the incident — write what caused it and what fixed it.
3. Run the cascade again. The new card now carries the past fix.

That's the core of Aegis.

---

### Path B — Full Splunk integration

Complete Path A first, then add Splunk and the agent.

**B1. Splunk + HEC**

1. Install Splunk Enterprise (free trial). Open http://localhost:8000.
2. Create an index named `aegis`.
3. Create an HEC token: name `aegis`, index `aegis`, sourcetype auto.
4. Copy `configs\aegis.example.toml` → `configs\aegis.toml`, paste the token.
5. Verify: `cargo run --bin aegis-daemon -- --check-hec`

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

Check Splunk: `index=aegis sourcetype=aegis:decision | head 5`

**B4. Dashboard**

Search & Reporting → Dashboards → Create New → name `Aegis` → Dashboard
Studio → Absolute layout → paste [`dashboards/aegis.json`](dashboards/aegis.json) → Save.

**B5. Splunk app** (optional)

```powershell
Copy-Item -Recurse apps\aegis_ai "$env:SPLUNK_HOME\etc\apps\aegis_ai"
& "$env:SPLUNK_HOME\bin\splunk" restart
```

AppInspect: **0 failures** ([report](apps/aegis_ai/appinspect-report.json)).

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
  Every SPL call goes through `tools/call` — fully auditable.

Wire Aegis into Cursor or Claude Desktop in two lines — see
[`docs/mcp.md`](docs/mcp.md).

---

## A note on Splunk Cloud access

Some Splunk Cloud features — specifically SLIM-backed Hosted Models and
CDTSM forecasting — require provisioning that wasn't available on the
14-day trial. We reached out to Splunk support and the hackathon channel
but didn't get access in time. The integrations are fully built and
wired; they activate the moment the environment is provisioned. We
didn't let it stop us — we routed through AITK + local Ollama to keep
the `| ai` pipeline live. Details in
[`docs/splunk-blocker.md`](docs/splunk-blocker.md).

---

## Troubleshooting

Common issues and fixes live in [`Troubleshooting.md`](Troubleshooting.md).
Quick hits:

* **`bind tcp listener ... already in use`** — another daemon is
  running. `Get-Process aegis-daemon | Stop-Process -Force`.
* **`HEC rejected events: 401`** — bad or disabled token. Re-issue in
  Splunk Web and update `configs/aegis.toml`.
* **Dashboard CDTSM panels show 404** — expected on local Enterprise;
  the rest of the dashboard works. See
  [`docs/splunk-blocker.md`](docs/splunk-blocker.md).

---

## License

[MIT](LICENSE).
