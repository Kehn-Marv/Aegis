# Aegis — Agentic Edge-Telemetry Gateway

[![License: MIT](https://img.shields.io/badge/license-MIT-3DDC97?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-stable-EB6228?style=flat-square&logo=rust)](Cargo.toml)
[![Python](https://img.shields.io/badge/python-3.11+-3776AB?style=flat-square&logo=python&logoColor=white)](sidecar/pyproject.toml)
[![MCP bidirectional](https://img.shields.io/badge/MCP-server%20%2B%20client-7C5CFF?style=flat-square)](docs/mcp.md)
[![splunklib.ai](https://img.shields.io/badge/splunklib.ai-Custom%20Alert%20%2B%20SPL%20Command-7C5CFF?style=flat-square)](apps/aegis_ai/README.md)
[![AppInspect](https://img.shields.io/badge/AppInspect-0%20failures-3DDC97?style=flat-square)](apps/aegis_ai/appinspect-report.json)
[![CDTSM](https://img.shields.io/badge/Hosted%20Models-CDTSM%20forecast-7C5CFF?style=flat-square)](docs/cdtsm-forecast.md)
[![LLM](https://img.shields.io/badge/LLM-gpt--oss%3A20b%20via%20AITK%20%2F%20Ollama-7C5CFF?style=flat-square)](docs/aitk-ollama.md)

> **Splunk Agentic Ops Hackathon 2026** · Observability track ·
> targeting *Best Use of Splunk MCP Server*, *Best Use of Splunk
> Hosted Models*, *Best Use of Splunk Developer Tools*, and *Best of
> Observability*.

A **Splunk-native, MCP-bidirectional** observability middleware that
sits between your applications and Splunk. It deduplicates repetitive
error loops into lightweight metrics, forecasts its own saturation
with the **Cisco Deep Time Series Model**, ships a
**`splunk-appinspect`-clean Splunk app** that grades alerts through
`splunklib.ai.Agent`, and is controllable end-to-end over MCP — both
*by* external AI agents (Cursor, Claude Desktop) and *from* an
autonomous AegisOps agent that itself uses
`splunk_run_query` over MCP JSON-RPC.

## What's in this repo for the hackathon

| Pillar | What it is | Where |
|---|---|---|
|  **Edge gateway** | Rust daemon that hashes structural signatures, collapses repeat lines into metric events, buffers offline with anomaly-first drain | `gateway/`, [`docs/finops-math.md`](docs/finops-math.md) |
|  **Aegis AI Splunk App** | Splunkbase-shaped app with **Custom Alert Action** + `\| aegisreason` **Custom Search Command**, both running `splunklib.ai.Agent`. 0 AppInspect failures. | [`apps/aegis_ai/`](apps/aegis_ai/) |
|  **CDTSM forecast loop** | Dashboard panels + AegisOps prompt-hint loop using Splunk-Hosted CDTSM to predict `queue_depth` and `dedup_savings_pct` 15 min ahead | [`docs/cdtsm-forecast.md`](docs/cdtsm-forecast.md) |
|  **MCP both ways** | Aegis hosts its own MCP server (`aegis-mcp`, 5 tools); AegisOps Agent is a real MCP client of `splunk_run_query` via JSON-RPC | [`docs/mcp.md`](docs/mcp.md) |
|  **AegisOps Agent** | Autonomous Python agent (observe → reason → act) with three live LLM transports: raw Ollama, AITK-routed Ollama, true Splunk Hosted Models | [`agent/README.md`](agent/README.md) |
|  **Dashboard Studio** | 11-panel dashboard with the two new CDTSM forecast lines | [`dashboards/aegis.json`](dashboards/aegis.json) |

**Where to look first:**

* [`ARCHITECTURE.md`](ARCHITECTURE.md) — root-level technical diagram
* [`docs/architecture.md`](docs/architecture.md) — deep-dive with data flows
* [`docs/finops-math.md`](docs/finops-math.md) — verifiable cost-savings worked example (99.96% reduction)
* [`docs/mcp.md`](docs/mcp.md) — Aegis MCP server + AegisOps as MCP client of Splunk MCP
* [`docs/aitk-ollama.md`](docs/aitk-ollama.md) — AITK Connection Management + local Ollama for live `| ai` SPL
* [`docs/cdtsm-forecast.md`](docs/cdtsm-forecast.md) — CDTSM dashboard panels and the agent feedback loop
* [`docs/saia-integration.md`](docs/saia-integration.md) — using Splunk AI Assistant 2.0 alongside Aegis
* [`docs/splunk-blocker.md`](docs/splunk-blocker.md) — Splunk Hosted Models SLIM-trial blocker and the two live workarounds
* [`apps/aegis_ai/README.md`](apps/aegis_ai/README.md) — Splunk app docs + AppInspect status
* [`agent/README.md`](agent/README.md) — autonomous AegisOps agent (observe → reason → act)

It is built for two failure modes the rest of the observability stack ignores:

1. **The cost crisis.** A crash-looping service emits the same 50-line stack
   trace 10,000 times a minute. Aegis sends the first occurrence in full and
   then collapses the rest into `{signature, count, window}` metrics —
   ~99.96% reduction in our worked example
   ([`docs/finops-math.md`](docs/finops-math.md)).
2. **The connectivity crisis.** At an edge site with intermittent uplink,
   blindly piping logs over the wire either drops data or saturates the link.
   Aegis buffers locally and, when the link returns, drains its queue
   *anomaly-first* so critical signals reach Splunk before routine summaries.

## Who is this for

**Buyer:** Platform / FinOps lead at a company running 50–500
microservices on Kubernetes (or edge IoT gateways) with a Splunk
Enterprise or Cloud deployment and a monthly ingest bill they need to
control.

**Workload:** A regional payment cluster (`us-east`, `eu-west`) where
routine INFO traffic is 95% of volume but crash-looping dependencies
during an outage can spike ingest 100× overnight.

**Outcome:** Aegis deployed as a DaemonSet (or systemd service) on each
node/region cuts repetitive error spam by ~99.96% at the edge
([`docs/finops-math.md`](docs/finops-math.md)), keeps anomalies
first-in-line during uplink loss, lets an autonomous **AegisOps
Agent** (`agent/`) watch the fleet and act via three live LLM
transports — raw Ollama (`gpt-oss:20b`, edge-first default), AITK +
Ollama via the `| ai` SPL command, or true Splunk Hosted Models when
SLIM is provisioned — and pairs with the **Aegis AI Splunk app**
(`apps/aegis_ai/`) that uses `splunklib.ai.Agent` to grade each
alert through a Custom Alert Action and a `| aegisreason` Custom
Search Command (AppInspect: 0 failures).

This is not a generic log forwarder. It is **FinOps guardrails +
predictive + agentic edge control** purpose-built for Splunk's
Observability track.

## Architecture

The root-level summary lives in [`ARCHITECTURE.md`](ARCHITECTURE.md). For
the full deep-dive see [`docs/architecture.md`](docs/architecture.md).

```
Microservice ──raw──▶ Aegis Gateway ──processed──▶ Splunk HEC ──▶ Splunk Core
                          │  ▲                                         │
                          │  └──MCP commands──┐                        ▼
                          │                   │                 Dashboards
                          ▼                   │              (incl. CDTSM forecast)
                   Python AI Sidecar    External AI Agent              │
                  (embeddings, cluster) (Cursor / Claude Desktop)      │
                                                                       ▼
                  AegisOps Agent (autonomous observe→reason→act)  ◀──── │ ai SPL via AITK
                  └─ LLM transport: ollama (default) │ aitk_ollama │ splunk_ai
                  └─ Splunk client:  REST oneshot   │ MCP tools/call (auto-detect)
                  └─ Reads CDTSM forecast → "predictive signal" hint to LLM
                  └─ Audits decisions to index=aegis sourcetype=aegis:agent

                  Aegis AI Splunk App  (apps/aegis_ai/, AppInspect clean)
                  └─ Custom Alert Action  ─┐
                  └─ |aegisreason CSC      ├─ splunklib.ai.OpenAIModel + Agent
                                            └─ base_url → Ollama / AITK / Hosted
```

## Repository layout

```
.
├── gateway/                 # Rust workspace (data plane + MCP control plane)
│   ├── aegis-core/          # ingest, dedup, summarize, queue, HEC, self-metrics
│   ├── aegis-mcp/           # rmcp server exposing edge control tools
│   └── aegis-daemon/        # binary that wires core + mcp together
├── sidecar/                 # Python AI sidecar (embeddings, clustering, hosted-model adapter)
├── ui/                      # React + Vite + Tailwind control panel
├── dashboards/              # Splunk Dashboard Studio JSON (11 panels + 2 CDTSM forecasts)
├── apps/
│   └── aegis_ai/            # Splunkbase-shaped app: splunklib.ai Custom Alert Action + | aegisreason CSC
├── agent/                   # AegisOps autonomous agent (observe → reason → act)
│   └── aegis_ops/
│       ├── splunk_mcp_client.py   # JSON-RPC 2.0 MCP client for Splunk MCP Server
│       └── transports.py          # ollama / splunk_ai / aitk_ollama
├── demo/                    # log spammer + canned smoke-test payloads + multi-edge launcher
├── configs/                 # example configuration files (demo, live, multi-edge)
├── dist/                    # build artefacts (Splunk app tarball; gitignored)
└── docs/                    # architecture, MCP, AITK+Ollama, CDTSM forecast, FinOps math, SAIA notes
```

---

# Setup

There are three setup paths depending on what you want to see:

* **Path A — Demo mode** — runs without Splunk. Best for understanding
  what the gateway does. **~5 minutes**, only needs Rust + Python 3.
* **Path B — Live mode (real Splunk)** — wires Aegis into a Splunk
  Enterprise instance with HEC + the Aegis AI Splunk app + the AITK
  `| ai` SPL command. Best for the actual production use case and for
  the agentic / dashboard demos. **~30 minutes** if you also have to
  install Splunk. See also
  [`docs/aitk-ollama.md`](docs/aitk-ollama.md) for the AITK + Ollama
  setup and [`apps/aegis_ai/README.md`](apps/aegis_ai/README.md) for
  the `splunklib.ai` app.
* **Path C — Full stack (multi-edge + AegisOps Agent)** — two regional
  gateways plus the autonomous agent loop, optionally with the
  CDTSM forecast loop and MCP-routed observations. Requires Path B
  credentials (Splunk auth token + HEC). See
  [Path C](#path-c--full-stack-multi-edge--aegisops-agent).

Both paths share the same prerequisites below. Add Splunk + an HEC
token for Path B. Add Node and/or Python sidecar packages for the
optional overlays in either path.

## Prerequisites (both paths)

| Tool                  | Minimum version       | How to verify                          |
|-----------------------|-----------------------|----------------------------------------|
| Rust (with Cargo)     | 1.80                  | `rustc --version` → `rustc 1.80+`      |
| Python                | 3.11                  | `python --version` → `Python 3.11+`    |
| Git                   | any recent            | `git --version`                        |
| A C compiler          | MSVC Build Tools (Windows) or `build-essential` (Linux) | Required for `rusqlite`'s bundled SQLite build |

> **Windows tip:** install Rust from <https://rustup.rs>. On first
> `cargo build` it will prompt for "Microsoft C++ Build Tools" if you
> don't have them; click yes and let it install.

Then clone and check the workspace builds and tests pass:

```powershell
git clone https://github.com/<your-handle>/aegis
cd aegis
cargo test --workspace
# Expect: 13 Rust tests passing (aegis-core)
```

If you see 13 passing Rust tests, the foundation is solid and you can pick
your path.

---

## Path A — Demo mode (no Splunk required, ~5 minutes)

This path runs the gateway in its **stderr-sink fallback mode**: no
Splunk endpoint configured, so processed events print to the terminal
instead of being shipped. You'll see dedup happen in real time and can
exercise the REST and MCP control planes.

### A1. Build the daemon

```powershell
cargo build --bin aegis-daemon
# First build takes ~1 minute (pulling deps), subsequent builds are seconds.
```

### A2. Start the daemon with the demo config

The demo config ([`configs/aegis.demo.toml`](configs/aegis.demo.toml))
uses a **3-second dedup window** so you see window-close events almost
immediately, omits the `[hec]` block (no Splunk required), and enables
the REST API + MCP HTTP server on `127.0.0.1:7321`.

```powershell
# In Terminal 1, leave this running:
.\target\debug\aegis-daemon.exe --config configs\aegis.demo.toml
```

Wait for these log lines (~3 seconds after startup):

```
INFO HEC unavailable or no queue; using stderr sink (demo mode)
INFO pipeline running window_secs=3 max_open=4096
INFO tcp ingest listening addr=127.0.0.1:5140
INFO MCP HTTP listening at 127.0.0.1:7321/mcp
INFO Control API at 127.0.0.1:7321/api/status
```

If port 5140 or 7321 is already in use, change them in
`configs/aegis.demo.toml` and restart.

### A3. Send the gateway some traffic

In a second terminal:

```powershell
# 50 crash-loops per second × 5 lines per crash × 5 seconds = 1,250 lines
python demo\log_spammer.py --target tcp://127.0.0.1:5140 --pattern crashloop --rate 50 --duration 5
```

### A4. What you'll see

Back in Terminal 1, after the spammer finishes and the 3-second window
closes, the daemon prints something like:

```
[FIRST  sig=80b287c5d8dd src=tcp://127.0.0.1:57239] ERROR [<TS>] payment-service: connection refused to <IP>:<N> (rid=<HEX>)
[FIRST  sig=8076f90b6a13 src=tcp://127.0.0.1:57239]   at db::Pool::checkout (db.rs:142)
[FIRST  sig=d3f4f81e3ba2 src=tcp://127.0.0.1:57239]   at handlers::charge (handlers.rs:88)
[FIRST  sig=5d7733d9e11b src=tcp://127.0.0.1:57239]   at runtime::task::poll (runtime.rs:303)
[FIRST  sig=54f5f11d3ed9 src=tcp://127.0.0.1:57239]   caused by: io::Error: ConnectionRefused
[DEDUP  sig=d3f4f81e3ba2 x  238 in   5.0s]   at handlers::charge (handlers.rs:88)
[DEDUP  sig=80b287c5d8dd x  238 in   5.0s] ERROR [<TS>] payment-service: connection refused to <IP>:<N> (rid=<HEX>)
[DEDUP  sig=5d7733d9e11b x  238 in   5.0s]   at runtime::task::poll (runtime.rs:303)
[DEDUP  sig=54f5f11d3ed9 x  238 in   5.0s]   caused by: io::Error: ConnectionRefused
[DEDUP  sig=8076f90b6a13 x  238 in   5.0s]   at db::Pool::checkout (db.rs:142)
```

That's **5 first-occurrence events** (each unique stack frame) plus
**5 collapsed metric events**, replacing the 1,250 raw lines. ~99.2%
reduction, with full incident context preserved in the first
occurrences.

### A5. Exercise the control plane

The daemon is also serving REST and MCP. Verify the REST API in a third
terminal:

```powershell
curl.exe http://127.0.0.1:7321/api/status
# {"uptime_secs":54,"online":true,"override_active":false,...,"events_in":1250,"events_out":10,"dedup_savings_pct":99.2,...}
```

Flip the gateway into raw-passthrough mode for the next 15 seconds:

```powershell
# Write the command body to a file (PowerShell quoting is awkward for JSON):
'{"command":"override","seconds":15}' | Out-File -Encoding ascii demo\override.json
curl.exe -X POST -H "Content-Type: application/json" --data-binary "@demo\override.json" http://127.0.0.1:7321/api/command
# {"ok":true,"message":"override raw passthrough enabled for 15s"}
```

Re-run the spammer within those 15 seconds and you'll see `[RAW]`
events streaming 1:1 — no dedup — proving the override actually
mutated the running daemon's behavior. After 15 seconds the override
auto-releases and dedup resumes.

Smoke-test the MCP server (the channel AI agents would use):

```powershell
'{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"smoke","version":"0.0.1"}},"id":1}' | Out-File -Encoding ascii demo\mcp_init.json
curl.exe -X POST -H "Content-Type: application/json" -H "Accept: application/json, text/event-stream" --data-binary "@demo\mcp_init.json" http://127.0.0.1:7321/mcp
```

You should see the response advertise `serverInfo.name: "aegis-mcp"`
and the `instructions` text that Cursor/Claude Desktop would surface
to their LLMs.

### A6. Stop everything

`Ctrl+C` in the daemon terminal. The spammer auto-exits when its
`--duration` elapses.

---

## Path B — Live mode (with Splunk Enterprise, ~30 minutes)

This path runs Aegis against a real Splunk Enterprise instance. You'll
see real `sourcetype=aegis:metric` events land in Splunk, the
self-metrics dashboard fill in, and (optionally) the AI classifier tag
each event.

### B1. Get Splunk Enterprise + HEC token

1. **Install Splunk Enterprise** (free 60-day trial). Download from
   <https://www.splunk.com/en_us/download/splunk-enterprise.html> and
   run the installer. On Windows it's a `.msi`; on macOS/Linux follow
   their instructions.
2. **Log in** to Splunk Web at <http://localhost:8000> (default admin
   account is created during install).
3. **Apply a Developer License** if you want 6 months of access
   instead of 60 days. Go to
   *Settings → Licensing → Add License* and follow the prompts at
   <https://dev.splunk.com/enterprise/dev_license/>.
4. **Create an index for Aegis events.** *Settings → Indexes → New
   Index* → name it `aegis` → save with defaults.
5. **Enable HEC and create a token.**
    - *Settings → Data inputs → HTTP Event Collector*
    - Top-right: *Global Settings* → set *All Tokens* = `Enabled`,
      uncheck *Enable SSL* if you want plain HTTP (easier for local
      dev), or leave SSL on if you're comfortable using `verify_tls = false`.
    - Click *New Token*:
        - Name: `aegis`
        - Source type: *Automatic*
        - Index: `aegis` (select the one you just made)
        - *Review* → *Submit*
    - Copy the token value. It looks like `12345678-1234-1234-1234-1234567890ab`.
6. **Confirm the HEC endpoint.** It's usually
   `https://localhost:8088/services/collector/event` (or `http://` if
   you disabled SSL).

### B2. Configure Aegis

```powershell
Copy-Item configs\aegis.example.toml configs\aegis.toml
notepad configs\aegis.toml
```

Fill in the `[hec]` section:

```toml
[hec]
endpoint    = "https://localhost:8088/services/collector/event"
token       = "PASTE-YOUR-HEC-TOKEN-HERE"
index       = "aegis"
host        = "aegis-edge-01"
timeout_secs = 10
verify_tls  = false      # set true if you've configured a real CA cert
```

Leave the rest of the file at defaults; you can tune `[dedup]`,
`[summary]`, and `[mcp]` later.

### B3. Verify the HEC plumbing before running the pipeline

```powershell
cargo run --bin aegis-daemon -- --check-hec
# Expect:
# INFO sending HEC ping endpoint=https://localhost:8088/...
# INFO HEC ping accepted; check your Splunk for sourcetype=aegis:diagnostic
```

In Splunk Web, search:

```spl
index=aegis sourcetype=aegis:diagnostic
```

You should see one event with `kind=startup_ping`. If you don't:

* Bad token → response will say `Token disabled` or `Invalid token`.
* Wrong endpoint → connection refused. Confirm port 8088 is open and
  the URL ends with `/services/collector/event`.
* Self-signed cert error → set `verify_tls = false` in the config.

### B4. Run the live pipeline

```powershell
# Terminal 1 — Aegis daemon
cargo run --release --bin aegis-daemon
# logs:  HEC configured; using queue-backed sink
#        MCP HTTP listening at 127.0.0.1:7321/mcp
#        Control API at 127.0.0.1:7321/api/status

# Terminal 2 — log spammer (sustained traffic for a minute)
python demo\log_spammer.py --target tcp://127.0.0.1:5140 --pattern crashloop --rate 200 --duration 60
```

In Splunk you'll start seeing:

* `sourcetype=aegis:raw` — first-occurrence raw lines (one per stack frame)
* `sourcetype=aegis:metric` — dedup collapses (one per signature per window)
* `sourcetype=aegis:selfmetric` — gateway perf snapshots every 15s

Sanity SPL to confirm:

```spl
index=aegis sourcetype=aegis:metric
| stats sum(count) AS suppressed_lines, count AS metric_events
```

You should see a very high `suppressed_lines : metric_events` ratio —
that ratio *is* the FinOps story.

### B5. Import the dashboard

1. In Splunk Web: *Dashboards → Create New Dashboard → Dashboard Studio*.
2. Click the source-editor `{ }` icon and paste the contents of
   [`dashboards/aegis.json`](dashboards/aegis.json) over the
   placeholder.
3. Save with whatever name you like. Set time range to *Last 15
   minutes* and auto-refresh to 5s.

You now have live panels for dedup savings, top suppressed signatures,
AI classifier verdict, classifier-strategy breakdown, and
first-occurrence rate. See [`dashboards/README.md`](dashboards/README.md)
for an SPL crib sheet.

---

## Path C — Full stack (multi-edge + AegisOps Agent)

This path demonstrates the **autonomous agent loop** — the centerpiece
for the Observability track. Two regional gateways (`us-east`,
`eu-west`) run in parallel; the AegisOps agent polls both, optionally
queries Splunk for trends, **reasons with a local Ollama LLM
(default) or Splunk Hosted Models when provisioned**, and actuates
low-risk decisions (`diagnostic`) while logging everything to
`sourcetype=aegis:agent`.

> **About the LLM transport.** The hackathon's Splunk Cloud 14-day
> trial does not provision the SLIM API that Splunk Hosted Models run
> on. We pivoted to **Ollama as the default LLM transport**, running
> the same `gpt-oss:20b` model identifier locally next to the edge
> gateway. The Splunk `| ai` integration is preserved as a hibernated
> transport — see [`docs/splunk-blocker.md`](docs/splunk-blocker.md).

**Prerequisites:** Path B (HEC) is optional. Ollama is mandatory.

| Prerequisite | Where to get it |
|--------------|-----------------|
| **Ollama** (mandatory) | <https://ollama.com/download>, then `ollama pull gpt-oss:20b` (~13 GB on disk, ~16 GB RAM — matches the `gpt-oss-20b` Splunk Hosted Models identifier). Smaller-RAM alternatives: `qwen2.5:7b` (~5 GB), `qwen2.5:3b` (~3 GB, explicitly tuned for JSON), `gemma2:2b` (~2 GB), `qwen2.5:1.5b` (~1.5 GB). Set the picked model in `[llm.ollama].model` |
| Splunk auth token (optional, lights up SPL observations) | *Settings → Tokens → New Token* with `search` capability |
| Splunk HEC token (optional, lights up agent audit trail) | *Settings → Data inputs → HTTP Event Collector* |
| Splunk SLIM API (only for `[llm].transport="splunk_ai"`) | **Currently trial-gated.** See [`docs/splunk-blocker.md`](docs/splunk-blocker.md) |

### C1. Launch two gateways (demo without Splunk)

Credential-free smoke test — stderr sink, no Splunk required:

```powershell
.\demo\run-multi-edge.ps1
curl.exe http://127.0.0.1:7321/api/status
curl.exe http://127.0.0.1:7322/api/status
```

Send different traffic to each region:

```powershell
python demo\log_spammer.py --target tcp://127.0.0.1:5140 --pattern crashloop --rate 50 --duration 10
python demo\log_spammer.py --target tcp://127.0.0.1:5142 --pattern routine --rate 200 --duration 10
```

### C2. Launch two gateways (live with Splunk)

```powershell
Copy-Item configs\aegis.us-east.example.toml configs\aegis-us-east.toml
Copy-Item configs\aegis.eu-west.example.toml configs\aegis-eu-west.toml
# Fill in HEC tokens in both files (host is already us-east / eu-west)

# Terminal 1
cargo run --release --bin aegis-daemon -- --config configs\aegis-us-east.toml

# Terminal 2
cargo run --release --bin aegis-daemon -- --config configs\aegis-eu-west.toml
```

### C3. Configure and run the AegisOps agent

```powershell
cd agent
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e .
Copy-Item configs\aegis-ops.example.toml configs\aegis-ops.toml
# (Defaults are pure-Ollama, zero Splunk creds. Edit only to add Splunk.)
aegis-ops run --config configs\aegis-ops.toml --once -v
```

The first tick takes ~10 seconds (Ollama loads the model). Subsequent
ticks are sub-second. Expected output:

```
INFO AegisOps starting: 2 gateway(s), policy=low_risk_auto, dry_run=False, llm=ollama, splunk=off, audit=off
INFO [us-east] decision=noop(-) conf=0.95 exec=auto      | gateway healthy, no actionable signal
INFO [eu-west] decision=noop(-) conf=0.95 exec=auto      | gateway healthy, no actionable signal
```

Dry-run for prompt iteration (no actuation, no HEC):

```powershell
aegis-ops run --config configs\aegis-ops.toml --dry-run --once -v
```

To light up SPL observations and audit, edit `[splunk]` and `[audit]`
in the config. To switch from Ollama to Splunk Hosted Models (when
SLIM access is available), change `[llm].transport` to `"splunk_ai"`.

Verify audit trail in Splunk (when `[audit]` is configured):

```spl
index=aegis sourcetype=aegis:agent
| table _time, gateway, decision.action, exec_mode, decision.confidence, decision.justification
| sort -_time
```

See [`agent/README.md`](agent/README.md) for transport / policy
details and [`docs/saia-integration.md`](docs/saia-integration.md) for
pairing with Splunk AI Assistant 2.0.

---

## Optional overlays (work with either path)

### O1. Python AI sidecar (semantic classification)

Adds the `classification: {label, confidence, strategy}` field to every
collapsed event. Required for routine-traffic summarization.

```powershell
cd sidecar
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e .
# Optional: pip install -e ".[dev]"  for pytest
python -m aegis_sidecar.server
# logs: Uvicorn running on http://127.0.0.1:8765
```

Then in `configs/aegis.toml` (or `configs/aegis.demo.toml`) flip:

```toml
[sidecar]
url = "http://127.0.0.1:8765"
enabled = true

[summary]                    # optional: roll routine collapses into Summary events
enabled = true
```

Restart the daemon and the next collapsed events in Splunk will carry
a `classification` block. See [`sidecar/README.md`](sidecar/README.md)
for the Splunk Hosted Model adapter env vars.

**Splunk Hosted Models (preferred):** set `AEGIS_SPLUNK_URL` and
`AEGIS_SPLUNK_TOKEN` to classify via SPL `| ai` — the same transport
the AegisOps agent uses. See [`sidecar/README.md`](sidecar/README.md).

**OpenAI-compatible fallback:** set `AEGIS_HOSTED_MODEL_URL` for local
vLLM / Ollama during offline development.

> **First-call note:** the sidecar lazy-downloads
> `sentence-transformers/all-MiniLM-L6-v2` (~80 MB) on the first
> `/embed` or `/classify` request. If the machine is offline the
> sidecar falls back to a deterministic hash-based embedding so the
> API stays functional.

### O2. Control-panel UI (browser dashboard)

Single-page React app that polls `/api/status` every 2 s and dispatches
control commands. **Requires Node.js 20+.**

```powershell
cd ui
npm install        # first run only
npm run dev
# Vite serves on http://localhost:5173 and proxies /api & /mcp to the daemon at :7321
```

Open <http://localhost:5173>. Three KPI tiles update live, the
Online/Offline toggle and the Remote MCP Command console post to the
daemon. See [`ui/README.md`](ui/README.md).

### O3. Cursor / Claude Desktop MCP client

Lets an AI agent inspect and control the running gateway in natural
language. Detailed config snippets in [`docs/mcp.md`](docs/mcp.md), but
the short version for Cursor — add to `%USERPROFILE%\.cursor\mcp.json`:

```json
{
  "mcpServers": {
    "aegis": {
      "url": "http://127.0.0.1:7321/mcp"
    }
  }
}
```

Reload Cursor's MCP settings; you should now see five Aegis tools
(`status`, `reset`, `diagnostic`, `override`, `replay_raw`) in the
tool picker.

---

## MCP control plane

Aegis registers itself as an MCP server exposing the following tools,
callable from any MCP-aware AI agent:

| Tool          | Description                                                        |
|---------------|--------------------------------------------------------------------|
| `status`      | Current queue depth, dedup ratio, online/offline state, uptime     |
| `reset`       | Clear the priority queue and in-memory dedup table                 |
| `diagnostic`  | Toggle verbose tracing at the edge for N seconds                   |
| `override`    | Disable compression and stream raw logs for N seconds              |
| `replay_raw`  | Re-emit buffered raw logs for a given time window (stub — see docs/mcp.md) |

The full Cursor / Claude Desktop / Splunk MCP Server orchestration
guide lives in [`docs/mcp.md`](docs/mcp.md).

---

## Troubleshooting

| Symptom | Fix |
|---|---|
| `cargo build` fails with a linker / `link.exe` error on Windows | Install MSVC Build Tools (the cargo error message has the link). |
| Daemon prints `bind tcp listener at 127.0.0.1:5140 ... Os { code: 10048 ... }` | Another process is already on port 5140 (often a previous Aegis daemon you forgot to kill). `Get-Process aegis-daemon \| Stop-Process -Force`, then restart. |
| Daemon prints `bind aegis http listener at 127.0.0.1:7321 ... already in use` | Same, but for the MCP/REST port. Edit `mcp.http_listen` in your config or kill the stale daemon. |
| `cargo run -- --check-hec` returns `HEC rejected events: 401` | Bad or disabled HEC token. Re-issue the token in Splunk Web and update `configs/aegis.toml`. |
| `cargo run -- --check-hec` returns a TLS error | Self-signed cert. Set `verify_tls = false` in `[hec]`. |
| Splunk search returns nothing even though daemon says `HEC batch delivered` | Check the *index* in your search matches `index=aegis` (or whatever you set). Check the time range covers when events landed. |
| `npm install` in `ui/` is slow | First install is ~1 minute (82 packages). Subsequent runs are seconds. |
| Sidecar startup error: `ModuleNotFoundError: No module named 'aegis_sidecar'` | You're running `python server.py` directly. Use `pip install -e .` inside the `sidecar/` virtualenv, then `python -m aegis_sidecar.server`. |
| Sidecar takes ~30 s on first `/classify` call | Lazy-loading the sentence-transformer model (~80 MB). Subsequent calls are sub-millisecond. |
| UI shows "UNREACHABLE" badge | Daemon isn't running, or its MCP/REST port differs from `7321`. Confirm the daemon log says `Control API at 127.0.0.1:7321/api/status`. |

---

## License

[MIT](LICENSE).
