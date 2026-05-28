# Aegis

> **Aegis stops the noise, finds what broke first, and remembers every incident
> your team has ever had — so when it happens again, it tells you exactly what
> fixed it last time.**

[![License: MIT](https://img.shields.io/badge/license-MIT-3DDC97?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-EB6228?style=flat-square&logo=rust)](Cargo.toml)
[![Python](https://img.shields.io/badge/python-3.11+-3776AB?style=flat-square&logo=python&logoColor=white)](sidecar/pyproject.toml)
[![Splunk MCP](https://img.shields.io/badge/MCP-server+client-7C5CFF?style=flat-square)](docs/mcp.md)
[![AppInspect](https://img.shields.io/badge/AppInspect-0%20failures-3DDC97?style=flat-square)](apps/aegis_ai/appinspect-report.json)

Splunk Agentic Ops Hackathon 2026 · Observability track.

---

## What Aegis does, in plain English

Aegis sits between your services and Splunk. It does four things:

1. **Stops the noise.** A service crash-loops and fires the same error 10,000
   times. Aegis sends *one* full copy and collapses the rest into a single
   count, so your ingest bill stops exploding during outages.

2. **Finds what broke first.** When several services start failing in the same
   window, Aegis picks the one that broke earliest and says *that's the root
   cause; everything after it is collateral damage.* Output is one sentence,
   not 200 alerts.

3. **Remembers every incident.** When the engineer fixes it, Aegis asks two
   questions — *what was the cause?* and *what fixed it?* — and stores both.
   Six months later when the same shape happens again, Aegis surfaces the
   fix that worked last time.

4. **Gives the engineer one focused decision moment.** Instead of an "Execute"
   button that prods production, Aegis shows a single card: root cause,
   similar past fix, suggested next step, and three buttons — `I'm on it`,
   `Show me more`, `This looks different`. The engineer is in control.

It's local, free, and fast. SQLite for memory. Rust for the hot path. No
external services required.

```text
                          ┌─────────────┐
   apps ──raw logs──▶     │   Aegis     │  ──processed events──▶  Splunk
                          │   gateway   │
                          └──────┬──────┘
                                 │
                          decision card (UI / MCP / Splunk dashboard)
```

---

## A 30-second demo (no Splunk required)

You need Rust 1.80+ and Python 3.11+. Then, in three terminals:

```powershell
# 1. Build and start the gateway in demo mode (no Splunk needed).
cargo build --bin aegis-daemon
.\target\debug\aegis-daemon.exe --config configs\aegis.demo.toml
```

```powershell
# 2. In a second terminal, send a multi-service incident.
python demo\log_spammer.py --target tcp://127.0.0.1:5140 --pattern cascade
```

```powershell
# 3. In a third terminal, watch the decision card appear.
curl.exe --silent http://127.0.0.1:7321/api/decision
```

You'll see something like:

```json
{
  "state": "red",
  "root_cause_service": "payment-api",
  "headline": "payment-api broke first. checkout followed 4s later. orders followed 8s later. Root cause: payment-api (100% confidence).",
  "business_impact": "Handles all transaction processing.",
  "suggested_next_step": "Check payment-api health and recent deploys...",
  "similar_incidents": []
}
```

Resolve it with two short sentences:

```powershell
'{"cause":"DB pool exhausted under retry storm.","fix":"Increased pool to 32, retry interval to 30s."}' | Out-File -Encoding ascii data\resolve.json
$id = (curl.exe -s http://127.0.0.1:7321/api/incidents | ConvertFrom-Json).incidents[0].id
curl.exe -X POST -H "Content-Type: application/json" --data-binary "@data\resolve.json" "http://127.0.0.1:7321/api/incidents/$id/resolve"
```

Re-run the cascade. The new decision card now carries the past fix:

```json
"suggested_next_step": "This looks 100% similar to a past incident (fixed in 2 min last time). Last time the cause was: \"DB pool exhausted under retry storm.\" The fix was: \"Increased pool to 32, retry interval to 30s.\" Start by verifying that."
```

That's Aegis. The institutional memory of every previous on-call, at the
fingertips of the new one.

---

## What's in this repo

```text
gateway/        Rust workspace (data plane + MCP control plane)
├── aegis-core/     noise gate, causal chain, incident memory, decision engine
├── aegis-mcp/      MCP server + REST API
└── aegis-daemon/   binary that wires it all together
sidecar/        Optional Python service for embedding-based classification
agent/          AegisOps autonomous agent (observes the gateway, reasons with an LLM)
apps/aegis_ai/  Splunkbase-shaped Splunk app (Custom Alert Action + |aegisreason SPL)
ui/             React control panel — decision card hero + incident memory list
dashboards/     Splunk Dashboard Studio JSON
configs/        TOML configs (demo + live + multi-edge)
demo/           log_spammer.py traffic generator
docs/           deep-dive docs (architecture, MCP, FinOps math, CDTSM, …)
```

The first place to look:

| Topic                  | File                                                                |
|------------------------|---------------------------------------------------------------------|
| Architecture           | [`ARCHITECTURE.md`](ARCHITECTURE.md)                                 |
| Causal chain detection | [`docs/causal-chain.md`](docs/causal-chain.md)                       |
| Incident memory        | [`docs/memory.md`](docs/memory.md)                                   |
| Decision card          | [`docs/decision-card.md`](docs/decision-card.md)                     |
| FinOps math            | [`docs/finops-math.md`](docs/finops-math.md)                         |
| MCP bidirectional      | [`docs/mcp.md`](docs/mcp.md)                                         |
| Troubleshooting        | [`Troubleshooting.md`](Troubleshooting.md)                           |

---

## The setup paths

Pick the one that matches what you want to see. All three share the same
prerequisites: **Rust 1.80+**, **Python 3.11+**, and (on Windows) MSVC Build
Tools (Cargo will prompt you on first build).

| Path | What you get                                                               | Time          |
|------|----------------------------------------------------------------------------|---------------|
| **A** | The 30-second demo above. No Splunk. The four pillars work end-to-end.    | ~5 minutes    |
| **B** | Path A plus Splunk Enterprise: HEC ingest, AI sidecar, full dashboard.    | ~45 minutes\* |
| **C** | Path B plus two regional gateways and the autonomous AegisOps agent.      | +15 minutes   |

\* Most of Path B's time is installing Splunk Enterprise + AI Toolkit, not Aegis.

---

## Path A — Demo (no Splunk required)

```powershell
git clone https://github.com/<your-handle>/aegis
cd aegis
cargo test --workspace          # 51 Rust tests should pass
cargo build --bin aegis-daemon
.\target\debug\aegis-daemon.exe --config configs\aegis.demo.toml
```

In a second terminal, send traffic and watch the decision card:

```powershell
# A multi-service cascade (~10s): payment-api → checkout → orders
python demo\log_spammer.py --target tcp://127.0.0.1:5140 --pattern cascade

# Read the current decision card
curl.exe --silent http://127.0.0.1:7321/api/decision

# List remembered incidents
curl.exe --silent http://127.0.0.1:7321/api/incidents
```

Other patterns to try: `crashloop` (dedup demo), `routine` (idle traffic),
`silence` (silent-service detector demo).

**See the live control panel:**

```powershell
cd ui
npm install         # first time only, ~1 minute
npm run dev         # http://localhost:5173
```

The UI is built around the decision card. When state is green, it's quiet.
When red, the card takes the page over with root cause, business impact,
similar past incidents, and the three buttons. Below the card, the incident
memory panel lets you click any past chain and fill in a 2-line resolution
that becomes future Aegis intelligence.

---

## Path B — Live with Splunk Enterprise

### B1. Splunk + HEC

1. Install Splunk Enterprise (free 60-day trial). Open
   `http://localhost:8000` and log in.
2. **Settings → Indexes → New Index** → `aegis`.
3. **Settings → Data inputs → HTTP Event Collector** → *Global Settings*:
   enable, optionally disable SSL for local dev. *New Token*:
   - Name: `aegis`, Index: `aegis`, Sourcetype: *Automatic*.
   - Copy the token.

### B2. Configure Aegis

```powershell
Copy-Item configs\aegis.example.toml configs\aegis.toml
notepad configs\aegis.toml
```

Fill in `[hec].token`, leave the rest at defaults.

### B3. Verify HEC

```powershell
cargo run --bin aegis-daemon -- --check-hec
# Expect:  HEC ping accepted; check your Splunk for sourcetype=aegis:diagnostic
```

If you see a 401 or 404, [`Troubleshooting.md`](Troubleshooting.md) lists
the usual culprits (token, port 8088, self-signed TLS).

### B4. Install the AI sidecar (powers classifier panels)

```powershell
cd sidecar
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e . --extra-index-url https://download.pytorch.org/whl/cpu
python -m aegis_sidecar.server
# expect:  Uvicorn running on http://127.0.0.1:8765
```

Leave that terminal open.

### B5. Run the live pipeline

You need three terminals: sidecar (B4), daemon, spammer.

```powershell
# Terminal 2 — daemon, repo root:
cargo run --release --bin aegis-daemon

# Terminal 3 — send traffic
python demo\log_spammer.py --target tcp://127.0.0.1:5140 --pattern cascade
# (or --pattern crashloop --rate 200 --duration 60 for the FinOps story)
```

When you are done exploring Path B, **stop the B5 daemon** (Ctrl+C in
Terminal 2). It holds ports **5140** and **7321** that Path C needs for
the us-east gateway. If you skip this and later see
`bind ... already in use`, see [Port already in use](Troubleshooting.md#port-already-in-use-windows) in `Troubleshooting.md`.

In Splunk Web, **Search & Reporting** with time range *Last 15 minutes*:

```spl
# Did decision cards land?
index=aegis sourcetype=aegis:decision | head 5

# Causal chains
index=aegis sourcetype=aegis:causal | head 5

# Incident memory entries
index=aegis sourcetype=aegis:incident | head 5
```

### B6. Import the dashboard

Search & Reporting → **Dashboards → Create New Dashboard** → name `Aegis`,
**Dashboard Studio**, **Absolute** layout. Click the **`{ }`** icon in the
toolbar, paste the contents of [`dashboards/aegis.json`](dashboards/aegis.json),
**Apply and close**, **Save**.

All panels populate within seconds. The CDTSM forecast panels at the bottom
require **Splunk AI Toolkit** (Splunkbase) and a **Splunk Cloud** stack
with SLIM provisioning — see
[`docs/splunk-blocker.md`](docs/splunk-blocker.md). The other panels work
on local Enterprise.

### B7. Optional: install the Splunk app

[`apps/aegis_ai/`](apps/aegis_ai/) is a Splunkbase-shaped app that adds a
`|aegisreason` SPL command and a Custom Alert Action, both powered by
`splunklib.ai.Agent`. AppInspect passes with **0 failures, 0
future-failures** ([report](apps/aegis_ai/appinspect-report.json)).

```powershell
Copy-Item -Recurse apps\aegis_ai "$env:SPLUNK_HOME\etc\apps\aegis_ai"
& "$env:SPLUNK_HOME\bin\splunk" restart
```

See [`apps/aegis_ai/README.md`](apps/aegis_ai/README.md) for the LLM
configuration (Ollama by default).

---

## Path C — Multi-edge + AegisOps Agent

Complete Path B first. Then add Ollama and the agent:

```powershell
# C1 - install Ollama, pull a model
# https://ollama.com/download
ollama pull qwen2.5:3b      # ~3 GB RAM. For 16 GB+ machines: ollama pull gpt-oss:20b

# C2 - two regional gateways
# Stop any daemon still running from Path B5 (same ports as us-east)

Copy-Item configs\aegis.us-east.example.toml configs\aegis-us-east.toml
Copy-Item configs\aegis.eu-west.example.toml configs\aegis-eu-west.toml
# edit both files: paste your HEC token

# Terminal A:  cargo run --release --bin aegis-daemon -- --config configs\aegis-us-east.toml
# Terminal B:  cargo run --release --bin aegis-daemon -- --config configs\aegis-eu-west.toml

# C3 - the agent
cd agent
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e .
Copy-Item configs\aegis-ops.example.toml configs\aegis-ops.toml
# edit configs\aegis-ops.toml: set model = " " under [llm.ollama] to what you pulled, e.g (model = "qwen2.5:3b")
# paste Splunk + HEC tokens under [splunk] & [audit] respectively.

aegis-ops --config configs\aegis-ops.toml --once -v   # Run the agent once, then exits (smoke test)
aegis-ops --config configs\aegis-ops.toml -v            # Runs continuously until you press Ctrl+C
```

What the agent does each tick:

```text
for each gateway:
    observation = REST /api/status + /api/decision + (optional) Splunk SPL
    decision    = LLM(observation)        # via Ollama, AITK | ai, or Splunk Hosted Models
    if policy says auto-actuate:
        gateway.POST /api/command         # only diagnostic / noop by default
    audit -> Splunk HEC, sourcetype=aegis:agent
```

The agent reads the gateway's own decision card and forwards it (with
similar past fixes) into the LLM prompt — so the LLM is grounded in the
gateway's vetted analysis, not asked to re-derive it.

See [`agent/README.md`](agent/README.md) for the transport matrix and
policy modes.

---

## MCP control plane

Aegis is on **both sides** of MCP:

* **Aegis as MCP server** at `http://127.0.0.1:7321/mcp`. Tools published:

  | Tool                 | What it does                                                  |
  |----------------------|---------------------------------------------------------------|
  | `status`             | Live gateway snapshot                                          |
  | `latest_decision`    | Current decision card (`null` if green)                        |
  | `recent_incidents`   | Top-N fingerprints from incident memory                        |
  | `resolve_incident`   | Attach a cause + fix card to a past incident                   |
  | `acknowledge`        | Mark current decision as "I'm on it"                           |
  | `diagnostic` / `override` / `reset` / `replay_raw` | Bounded-window observability tools |

* **AegisOps Agent as MCP client** of the official Splunk MCP Server.
  Every observational SPL call traverses `tools/call` so the agent's
  reasoning is fully auditable in `index=_internal sourcetype=mcpjson`.

Wire Aegis into Cursor or Claude Desktop in two lines — see
[`docs/mcp.md`](docs/mcp.md).

---

## Troubleshooting

Common issues and fixes live in [`Troubleshooting.md`](Troubleshooting.md).
Greatest hits:

* **`bind tcp listener ... already in use`** — another `aegis-daemon`
  process is still alive. `Get-Process aegis-daemon | Stop-Process -Force`.
* **`HEC rejected events: 401`** — bad or disabled HEC token. Re-issue in
  Splunk Web and update `configs/aegis.toml`.
* **Dashboard CDTSM panels error with `404`** — expected on local Splunk
  Enterprise; the rest of the dashboard still works. See
  [`docs/splunk-blocker.md`](docs/splunk-blocker.md).
* **Decision card never goes red in the demo** — make sure you're running
  the `cascade` pattern, and that the cascade fits inside the configured
  `[causal].window_secs` (30s default in demo configs).

---

## License

[MIT](LICENSE).
