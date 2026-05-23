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
* **Path B — Live mode (real Splunk)** — single edge gateway wired into
  Splunk Enterprise: HEC ingest, AI sidecar classification, AITK/CDTSM
  forecast panels, and the full 11-panel dashboard. **~45 minutes**
  if you also have to install Splunk + AITK. This is the **complete
  demo** — every panel populates when you finish.
* **Path C — Full stack (multi-edge + AegisOps Agent)** — builds on
  Path B: two regional gateways plus the autonomous agent loop that
  observes, reasons (Ollama), and actuates. **Do Path B first**, then
  continue into C without shutting Splunk down. See
  [Path B vs Path C](#path-b-vs-path-c--which-one-do-i-need).

Path A shares the Rust/Python prerequisites below. Path B adds Splunk,
HEC, the AI sidecar, and Splunk AI Toolkit. Path C adds Ollama and the
AegisOps agent on top of a completed Path B.

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

## Path B — Live mode (with Splunk Enterprise, ~45 minutes)

This path runs the **full Aegis stack** against real Splunk Enterprise:
HEC ingest, AI sidecar classification, CDTSM forecast panels, and the
complete 11-panel dashboard. Every AI feature is wired into the setup
flow — nothing is left as an optional overlay.

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

In Splunk Web (`http://localhost:8000`):

1. From the home page, open the **Search & Reporting** app (default
   app; dark/green tile on the left).
2. In the search bar at the top, run:

```spl
index=aegis sourcetype=aegis:diagnostic
```

3. Set the time range to *Last 24 hours* (or wider) and click **Search**.

You should see one event with `kind=startup_ping`. If you don't:

* Bad token → response will say `Token disabled` or `Invalid token`.
* Wrong endpoint → connection refused. Confirm port 8088 is open and
  the URL ends with `/services/collector/event`.
* Self-signed cert error → set `verify_tls = false` in the config.

### B3.5 Install the AI sidecar (required — powers classifier panels)

The sidecar adds `classification.label` and `classification.strategy`
to every collapsed metric event. Without it, the **AI classifier
verdict** and **Classifier strategy used** dashboard panels stay empty.

One-time install:

```powershell
cd sidecar
python -m venv .venv
.\.venv\Scripts\Activate.ps1
# PyTorch (~123 MB) often fails mid-download from PyPI on Windows.
# Install it from the official CPU wheel index first, then the rest:
pip install torch --index-url https://download.pytorch.org/whl/cpu
pip install -e .
```

[`configs/aegis.toml`](configs/aegis.example.toml) already ships with
`[sidecar] enabled = true` — no config edit needed. You will start the
sidecar in **B4 Terminal 1** and leave it running.

> **First-call note:** the sidecar lazy-downloads
> `sentence-transformers/all-MiniLM-L6-v2` (~80 MB) on the first
> `/classify` request. If the machine is offline it falls back to a
> deterministic hash-based embedding so the API stays functional.

> **`win_amd64` on the PyTorch wheel?** That name means 64-bit Windows
> on x86-64 (Intel **or** AMD). It is the correct wheel for almost all
> modern Windows laptops — it does not require an AMD processor.

Verify the sidecar started:

```powershell
# Expect: Uvicorn running on http://127.0.0.1:8765
python -m aegis_sidecar.server
```

Leave that terminal open.

### B3.6 Install Splunk AI Toolkit (required — powers CDTSM forecast panels)

The two bottom dashboard panels (`Queue depth — 15-min forecast` and
`Dedup savings % — 15-min forecast`) use Splunk's `| apply CDTSM`
command. Without AI Toolkit installed you will see
`Unknown search command 'apply'`.

**Splunkbase** is just Splunk's app store. You do not open a separate
website — in Splunk Web go to **Apps → Find More Apps** and you land on
**Browse More Apps** (the page with the search box on the left and app
tiles in the middle).

Install two apps from that page, **one at a time**, restarting Splunk
after each:

#### App 1 — Python for Scientific Computing

> **Pick the Windows one.** You need **Python for Scientific Computing
> (for Windows 64-bit)** — not Linux, Mac Intel, or Mac Apple Silicon.

1. Splunk Web → top-left **Apps** dropdown → **Find More Apps**.
2. In the left sidebar search box (*Find apps by keyword…*), type:
   `Python for Scientific Computing`
3. Click **Python for Scientific Computing (for Windows 64-bit)** →
   green **Install**.
4. Follow the prompts → when it asks to restart, **restart Splunk**.

**If Install fails with `Winsock error 10054` or `Connection reset`:**
the package is very large and Splunk's built-in downloader often drops
the connection. Use the manual path instead:

1. In your **browser** (Chrome/Edge), open
   [Python for Scientific Computing — Windows 64-bit](https://splunkbase.splunk.com/app/2883)
   (log in with the same Splunk account if prompted).
2. Click **Download** and save the `.spl` or `.tgz` file to your
   Downloads folder. Wait for the full download to finish in the
   browser — do not interrupt it.
3. Back in Splunk Web → **Apps** (left sidebar) → **Manage Apps**.
4. Click **Install app from file** → **Choose File** → select the
   file you downloaded → **Upload**.
5. Restart Splunk when prompted (~1–2 minutes).

#### App 2 — Splunk AI Toolkit

1. Install PSC (App 1) first — AITK depends on it.
2. Either use **Find More Apps** → search `Splunk AI Toolkit` →
   **Install**, **or** if that also fails with a network error:
   - Download from
     [Splunk AI Toolkit on Splunkbase](https://splunkbase.splunk.com/app/2890)
   - **Apps → Manage Apps → Install app from file → Upload**
3. Restart Splunk when prompted.

#### Smoke-test AITK loaded

In **Search & Reporting**, paste into the search bar and click
**Search**:

```spl
| makeresults
| eval prompt="Reply with the single word pong."
| ai prompt=prompt
```

You should get back a row within a few seconds. If you see
`Unknown search command 'ai'`, AITK is not installed or Splunk needs
another restart.

The CDTSM forecast lines themselves need ~15 minutes of
`sourcetype=aegis:selfmetric` data (generated in B4) before they draw
a meaningful plot. See [`docs/cdtsm-forecast.md`](docs/cdtsm-forecast.md)
and [`docs/aitk-ollama.md`](docs/aitk-ollama.md) for deeper AITK wiring.

### B4. Run the live pipeline

You need **three terminals** open at the same time while traffic flows.
All three must be running **before** you start the log spammer.

| Terminal | Command | How to know it's up |
|---|---|---|
| **1 — Sidecar** | `python -m aegis_sidecar.server` | `Uvicorn running on http://127.0.0.1:8765` |
| **2 — Daemon** | `cargo run --release --bin aegis-daemon` | `tcp ingest listening`, `HEC configured`, `Control API at 127.0.0.1:7321/api/status` |
| **3 — Spammer** | `python demo\log_spammer.py ...` | Runs silently for 60s, then exits (normal) |

```powershell
# Terminal 1 — AI sidecar (start first; leave running)
cd sidecar
.\.venv\Scripts\Activate.ps1
python -m aegis_sidecar.server
# logs: Uvicorn running on http://127.0.0.1:8765

# Terminal 2 — Aegis daemon (leave running; repo root)
cargo run --release --bin aegis-daemon
# logs:  AI sidecar enabled url=http://127.0.0.1:8765
#        HEC configured; using queue-backed sink
#        MCP HTTP listening at 127.0.0.1:7321/mcp
#        Control API at 127.0.0.1:7321/api/status

# Terminal 3 — log spammer (repo root; one-shot — runs 60s then exits)
python demo\log_spammer.py --target tcp://127.0.0.1:5140 --pattern crashloop --rate 200 --duration 60
```

#### B4a. Check the daemon is up (before running the spammer)

Open a **fourth** PowerShell window (or use Terminal 3 before the
spammer) and run:

```powershell
Invoke-RestMethod http://127.0.0.1:7321/api/status
```

* **JSON comes back** (`online`, `events_in`, `dedup_savings_pct`, …)
  → daemon is running. Proceed to the spammer in Terminal 3.
* **"Unable to connect" / connection refused** → start Terminal 2
  (`cargo run --release --bin aegis-daemon`) and wait for the ingest /
  HEC log lines, then retry.

#### B4b. After the spammer finishes

1. Keep **Terminals 1 and 2** open — do not close the sidecar or daemon.
2. Wait **~60 seconds** so dedup metric windows flush and CDTSM panels
   accumulate selfmetric history.
3. Confirm classifications landed (sidecar must have been running
   **during** the spammer — it only tags **new** traffic; old Splunk
   events from before B3.5 will not have `classification.*` fields):

```spl
index=aegis sourcetype=aegis:metric "classification.label"=*
```

Set time range to *Last 15 minutes* in **Search & Reporting**. You
should see metric events with `classification.label` and
`classification.strategy`. Then refresh the dashboard — the **AI
classifier verdict** and **Classifier strategy used** panels should
populate.

**Already ran the spammer before starting the sidecar?** Leave
Terminals 1 and 2 running and re-run Terminal 3's spammer command once
both services are up, then wait 60s and search again.

### B4c. Search for the events in Splunk

The sourcetypes below are **labels on events**, not commands to run in
your terminal. You find them by typing an SPL query into Splunk's search
bar and clicking **Search**.

In Splunk Web (`http://localhost:8000`), open **Search & Reporting**
(default app; dark/green tile on the left), then:

1. Paste this into the search bar and click **Search**:

```spl
index=aegis
```

2. Set the time range to *Last 15 minutes* (or *Last 1 hour* if the
   daemon has been up longer).

You should see events. The landing page with an empty search bar shows
nothing until you run a query like the one above.

To filter by event type, run these one at a time in the same search
bar (not in PowerShell):

```spl
index=aegis sourcetype=aegis:raw
```

```spl
index=aegis sourcetype=aegis:metric
```

```spl
index=aegis sourcetype=aegis:selfmetric
```

What each sourcetype means:

* `aegis:raw` — first-occurrence raw lines (one per stack frame)
* `aegis:metric` — dedup collapses (one per signature per window)
* `aegis:selfmetric` — gateway perf snapshots every 15s

**Don't panic if you see very few events.** The `crashloop` pattern
generates thousands of identical lines; Aegis collapses most of them.
Seeing ~100 events while the daemon reports ~99% dedup savings is
exactly the FinOps story.

Sanity SPL to confirm dedup is working — paste into the search bar, set
the time range to *Last 15 minutes*, and click **Search**:

```spl
index=aegis sourcetype=aegis:metric
| stats sum(count) AS suppressed_lines, count AS metric_events
```

You should see a very high `suppressed_lines : metric_events` ratio —
that ratio *is* the FinOps story.

### B5. Import the dashboard

In Splunk Web (`http://localhost:8000`):

1. From the home page, click **Search & Reporting** in the left sidebar
   (same app you used for B3/B4 searches).
2. At the top of the page, open the **Dashboards** menu → **Create New
   Dashboard**.
   - Shortcut from the home page: under *Common Tasks*, the **Visualize
     your data** tile also leads to dashboard creation.

**On the *Create New Dashboard* dialog:**

3. **Dashboard Title** — type `Aegis` (required).
4. **Description** — optional; leave blank or e.g. `Aegis edge gateway`.
5. **Permissions** — leave as *Private* (fine for local dev).
6. Select **Dashboard Studio** (not Classic Dashboards).
7. **Select layout mode** — choose **Absolute** (`aegis.json` uses
   absolute layout; Grid will misalign the panels).
8. Click the green **Create** button.

**In the Dashboard Studio editor:**

9. In the toolbar above the canvas, click the **Terminal** icon — `{ }`
   on a document, immediately to the **left of the `?` help icon**.
10. Select all placeholder JSON, delete it, and paste the full contents
    of [`dashboards/aegis.json`](dashboards/aegis.json).
11. Click **Apply and close** (green button, top-right of the terminal
    editor). Back on the canvas, click **Save** (top-right) to persist
    the dashboard.

**View the dashboard:**

12. Open the saved dashboard. Set the time range to *Last 15 minutes*
    and auto-refresh to 5s so panels populate while the pipeline runs.
13. Keep **Terminals 1 and 2** (sidecar + daemon) running — panels read
    live data from `index=aegis` and stay empty if either service stops.

**All 11 panels should populate** when B3.5–B4 were followed:

| Panel group | Powered by |
|---|---|
| Headline KPIs, ingest chart, top signatures, first-occurrence rate | B4 daemon + HEC (always) |
| AI classifier verdict, Classifier strategy used | B3.5 sidecar (Terminal 1) |
| CDTSM forecast lines (bottom two panels) | B3.6 AI Toolkit + ~15 min of selfmetric data |

You now have live panels for dedup savings, top suppressed signatures,
AI classifier verdict, classifier-strategy breakdown, CDTSM forecasts,
and first-occurrence rate. See [`dashboards/README.md`](dashboards/README.md)
for an SPL crib sheet.

---

### Path B vs Path C — which one do I need?

| | **Path B** | **Path C** |
|---|---|---|
| **What you get** | One edge gateway → Splunk → full 11-panel dashboard with AI classification + CDTSM forecasts | Everything in B **plus** two regional gateways and the autonomous AegisOps agent (observe → reason → act) |
| **Splunk** | Required (install + HEC) | Reuses the same Splunk instance from B — **keep it running** |
| **Sidecar + AITK** | Required (built into B3.5–B3.6) | Same — keep sidecar running; AITK already installed |
| **Ollama** | Not required | **Required** — the agent uses it for reasoning |
| **Can I skip B and do C alone?** | — | **No** for the full demo. Complete **B1 through B5 first** so Splunk, HEC, sidecar, AITK, and the dashboard are all working. |
| **Do I shut everything down between B and C?** | — | **No.** Leave Splunk Web running. Stop the single B4 daemon (`Ctrl+C` in Terminal 2) when you're ready for C2's two regional daemons — but keep the sidecar (Terminal 1) running and Splunk up. |

**Typical flow:** finish Path B end-to-end → confirm the dashboard looks
impressive → scroll down to Path C → install Ollama → swap the single
daemon for two regional ones (C2) → start the AegisOps agent (C3).

---

## Path C — Full stack (multi-edge + AegisOps Agent)

> **Start here only after Path B is complete (B1–B5).** Path C adds the
> autonomous agent loop on top of the Splunk + sidecar + AITK foundation
> you already built. Keep Splunk running; keep the sidecar running.

This path demonstrates the **autonomous agent loop** — the centerpiece
for the Observability track. Two regional gateways (`us-east`,
`eu-west`) run in parallel; the AegisOps agent polls both, optionally
queries Splunk for trends (including CDTSM forecasts), **reasons with a
local Ollama LLM (default) or Splunk Hosted Models when provisioned**,
and actuates low-risk decisions (`diagnostic`) while logging everything
to `sourcetype=aegis:agent`.

> **About the LLM transport.** The hackathon's Splunk Cloud 14-day
> trial does not provision the SLIM API that Splunk Hosted Models run
> on. We pivoted to **Ollama as the default LLM transport**, running
> the same `gpt-oss:20b` model identifier locally next to the edge
> gateway. The Splunk `| ai` integration is preserved as a hibernated
> transport — see [`docs/splunk-blocker.md`](docs/splunk-blocker.md).

**Prerequisites:** Path B complete (Splunk + HEC + sidecar + AITK +
dashboard working). **Ollama is mandatory** for the agent.

| Prerequisite | Where to get it |
|--------------|-----------------|
| **Path B complete** | [B1–B5 above](#path-b--live-mode-with-splunk-enterprise-45-minutes) — Splunk, HEC, sidecar, AITK, dashboard |
| **Ollama** (mandatory) | <https://ollama.com/download>, then `ollama pull gpt-oss:20b` (~13 GB on disk, ~16 GB RAM — matches the `gpt-oss-20b` Splunk Hosted Models identifier). Smaller-RAM alternatives: `qwen2.5:7b` (~5 GB), `qwen2.5:3b` (~3 GB, explicitly tuned for JSON), `gemma2:2b` (~2 GB), `qwen2.5:1.5b` (~1.5 GB). Set the picked model in `[llm.ollama].model` |
| Splunk auth token (recommended — lights up SPL observations + CDTSM in agent) | *Settings → Tokens → New Token* with `search` capability |
| Splunk HEC token | Already configured in Path B |
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

Stop the single B4 daemon (`Ctrl+C` in its terminal) — C2 replaces it
with two regional gateways. **Keep the sidecar (B4 Terminal 1) and
Splunk running.**

```powershell
Copy-Item configs\aegis.us-east.example.toml configs\aegis-us-east.toml
Copy-Item configs\aegis.eu-west.example.toml configs\aegis-eu-west.toml
# Fill in HEC tokens in both files (host is already us-east / eu-west)

# Terminal 2 (was B4 daemon terminal)
cargo run --release --bin aegis-daemon -- --config configs\aegis-us-east.toml

# Terminal 4 (new)
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

Verify audit trail in Splunk (when `[audit]` is configured) — in
Splunk Web (`http://localhost:8000`), open **Search & Reporting**,
paste into the search bar, set the time range to *Last 24 hours*, and
click **Search**:

```spl
index=aegis sourcetype=aegis:agent
| table _time, gateway, decision.action, exec_mode, decision.confidence, decision.justification
| sort -_time
```

See [`agent/README.md`](agent/README.md) for transport / policy
details and [`docs/saia-integration.md`](docs/saia-integration.md) for
pairing with Splunk AI Assistant 2.0.

---

## Optional overlays (extras beyond Path B / C)

These are **add-ons**, not required for the core demo. Path B already
includes the AI sidecar and AITK/CDTSM setup in B3.5–B3.6.

### O1. Python AI sidecar — advanced configuration

Path B [B3.5](#b35-install-the-ai-sidecar-required--powers-classifier-panels)
covers the required sidecar install. Use this section for optional
tuning: hosted-model adapters, summarization, and Splunk `| ai`
transport. See [`sidecar/README.md`](sidecar/README.md).

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
| Splunk search returns nothing even though daemon says `HEC batch delivered` | Run the search in **Search & Reporting** (`http://localhost:8000`), not the HEC URL on port 8088. Check the *index* in your search matches `index=aegis` (or whatever you set). Check the time range covers when events landed. |
| `index=aegis` returns zero events after running the B4 log spammer | 1) Confirm Terminals 1 and 2 (sidecar + daemon) are still running. 2) Re-run the spammer while both are up: `python demo\log_spammer.py --target tcp://127.0.0.1:5140 --pattern crashloop --rate 200 --duration 60`. 3) Wait ~60 seconds after it finishes so dedup metric windows can flush. 4) In **Search & Reporting**, run `index=aegis` with time range *Last 15 minutes* and click **Search**. |
| Dashboard **AI classifier verdict** panel is empty | The sidecar only classifies **new** traffic. Follow [B4a–B4b](#b4-run-the-live-pipeline): 1) Terminal 1 — `python -m aegis_sidecar.server` (`Uvicorn running on http://127.0.0.1:8765`). 2) Terminal 2 — confirm daemon with `Invoke-RestMethod http://127.0.0.1:7321/api/status`. 3) Re-run the spammer while both are up. 4) Wait 60s. 5) Search `index=aegis sourcetype=aegis:metric "classification.label"=*` in **Search & Reporting**, then refresh the dashboard. |
| `Invoke-RestMethod http://127.0.0.1:7321/api/status` fails | Daemon not running. Start Terminal 2: `cargo run --release --bin aegis-daemon` from the repo root; wait for `tcp ingest listening` before retrying. |
| Splunk app install fails with `Winsock error 10054` or `Connection reset` | Large app download dropped mid-transfer. Download manually in your browser ([PSC Windows 64-bit](https://splunkbase.splunk.com/app/2883), [AI Toolkit](https://splunkbase.splunk.com/app/2890)), then **Apps → Manage Apps → Install app from file → Upload**. See [B3.6](#b36-install-splunk-ai-toolkit-required--powers-cdtsm-forecast-panels). |
| Dashboard CDTSM panels show `Unknown search command 'apply'` | AI Toolkit not installed. Complete [B3.6](#b36-install-splunk-ai-toolkit-required--powers-cdtsm-forecast-panels), restart Splunk, re-open the dashboard. Panels also need ~15 min of `aegis:selfmetric` data — leave the daemon running. |
| `npm install` in `ui/` is slow | First install is ~1 minute (82 packages). Subsequent runs are seconds. |
| Sidecar startup error: `ModuleNotFoundError: No module named 'aegis_sidecar'` | You're running `python server.py` directly. Use `pip install -e .` inside the `sidecar/` virtualenv, then `python -m aegis_sidecar.server`. |
| `pip install -e .` in `sidecar/` fails with `ConnectionResetError` while downloading `torch` | Transient network drop on the ~123 MB PyTorch wheel. Run `pip install torch --index-url https://download.pytorch.org/whl/cpu` first, then `pip install -e .` again — pip reuses cached packages. The `win_amd64` in the wheel name is correct for 64-bit Windows on Intel or AMD — it does not mean you need an AMD CPU. |
| Sidecar takes ~30 s on first `/classify` call | Lazy-loading the sentence-transformer model (~80 MB). Subsequent calls are sub-millisecond. |
| UI shows "UNREACHABLE" badge | Daemon isn't running, or its MCP/REST port differs from `7321`. Confirm the daemon log says `Control API at 127.0.0.1:7321/api/status`. |

---

## License

[MIT](LICENSE).
