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
pip install -e . --extra-index-url https://download.pytorch.org/whl/cpu
```

That single command installs the sidecar **and** pulls PyTorch from
Splunk's recommended CPU wheel index (sentence-transformers needs torch;
PyPI's default source often flakes on Windows mid-download).

If the download fails partway through (`ConnectionResetError`), retry
PyTorch alone, then finish the rest — pip reuses the cached wheel:

```powershell
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

Install two Splunkbase apps — **different methods** depending on size:

| App | Size | Install method |
|---|---|---|
| Python for Scientific Computing (Windows 64-bit) | ~800 MB | **CLI only** — web Install/Upload fails |
| Splunk AI Toolkit | ~30 MB | **Splunk Web** — Apps → Find More Apps → Install |

#### App 1 — Python for Scientific Computing (Windows 64-bit) — CLI

> Download the **Windows 64-bit** build — not Linux or Mac. **Do not**
> use Splunk Web's green Install button or Upload for this app — the
> package is too large and the web UI fails with network errors or
> `Internal Server Error`.

Open **PowerShell as Administrator**. Splunk must be **running** before
`install app`. Replace `YOUR_USER` / `YOUR_PASSWORD` with your Splunk
admin credentials.

1. In your browser, download from Splunkbase (log in if prompted):
   [Python for Scientific Computing — Windows 64-bit](https://splunkbase.splunk.com/app/2883)
2. Save the `.tgz` to your Downloads folder. Wait for the full download
   to finish (~800 MB).
3. Install via CLI (adjust the filename if yours differs):

```powershell
& "C:\Program Files\Splunk\bin\splunk.exe" start
& "C:\Program Files\Splunk\bin\splunk.exe" install app "$env:USERPROFILE\Downloads\python-for-scientific-computing-for-windows-64-bit_432.tgz" -update 1 -auth YOUR_USER:YOUR_PASSWORD
& "C:\Program Files\Splunk\bin\splunk.exe" restart
```

Expect: `App '...\python-for-scientific-computing-for-windows-64-bit_432.tgz' installed`

Verify in Splunk Web → **Apps → Manage Apps** — **Python for Scientific
Computing (for Windows 64-bit)** should appear.

#### App 2 — Splunk AI Toolkit — Splunk Web

Install PSC (App 1) first — AITK depends on it. At ~30 MB, the AI
Toolkit installs reliably through Splunk Web (no CLI needed).

1. Splunk Web → top-left **Apps** dropdown → **Find More Apps**.
2. In the left search box, type: `Splunk AI Toolkit`
3. Click **Splunk AI Toolkit** → green **Install** button.
4. When the **Install — Success** dialog appears, click **Done**.
5. Restart Splunk if prompted (~1 minute).

You should see **Splunk AI Toolkit (5.7.x)** under **Apps → Manage Apps**.

#### B3.6a. Grant AI command permission (one-time)

Before any AITK search works, your Splunk role needs the AI capability:

1. Splunk Web → **Settings** (gear, top-right) → **Roles**.
2. Click your role (usually **`admin`**).
3. Open the **Capabilities** tab.
4. Search for `apply_ai_commander` → check **`apply_ai_commander_command`**.
5. **Save**.

If you skip this, searches fail with:
`Error in 'ai' command: User does not have permission to use 'ai' command.`

#### B3.6b. Smoke-test CDTSM (Splunk Cloud only)

The Aegis dashboard uses **`| apply CDTSM`**, not the **`| ai`** LLM
command. CDTSM is a **Splunk-Hosted foundation model** — it calls the
same SLIM/tenant API as Splunk Hosted Models.

> **Local Splunk Enterprise (Developer License):** CDTSM will **not**
> run. You will see:
> `CDTSM: Failed to determine API endpoint: Failed to retrieve tenant info: HTTP 404 Not Found`
> That is the same infrastructure gate documented in
> [`docs/splunk-blocker.md`](docs/splunk-blocker.md) — SLIM-backed
> hosted models are **Splunk Cloud only**. Path B on local Enterprise
> still delivers **9 of 11 dashboard panels** (everything except the
> two CDTSM forecast lines). The integration is wired and ready; it
> activates when you point Splunk at a Cloud stack with CDTSM enabled.

On a **Splunk Cloud** stack with CDTSM provisioned, keep the **sidecar +
daemon** running (B4) so `aegis:selfmetric` data exists, then in
**Search & Reporting** (time range *Last 15 minutes*):

```spl
index=aegis sourcetype=aegis:selfmetric
| timechart span=1m latest(queue_depth) AS queue_depth
| apply CDTSM queue_depth time_field=_time forecast_k=15 conf_interval=90 show_input=true
```

Use a **colon** in the sourcetype (`aegis:selfmetric`), not an underscore.

**Success:** a table/chart with `queue_depth` and `predicted(queue_depth)`
columns — no `Unknown search command 'apply'`.

**No data yet:** run B4 first (daemon + spammer), wait ~15 minutes, retry.

Refresh the Aegis dashboard — the bottom two CDTSM forecast panels populate
on Cloud; on local Enterprise they will show the tenant/404 error until
you migrate to a provisioned Cloud stack.

#### CDTSM vs `| ai` vs Ollama — three different things

| Feature | SPL command | What it does | Works on local Enterprise? |
|---|---|---|---|
| **CDTSM forecast** | `\| apply CDTSM` | Predicts `queue_depth` / `dedup_savings_pct` 15 min ahead (time-series model) | **No** — Splunk Cloud / SLIM only |
| **LLM chat / reasoning** | `\| ai prompt=…` | Text generation, policy decisions, classifier via hosted model | **Yes** — with Ollama wired through AITK Connection Management |
| **Sidecar classifier** | _(none — HTTP to sidecar)_ | Labels logs as routine/anomaly | **Yes** — already working on your dashboard |

**Ollama cannot substitute for CDTSM.** They solve different problems:
CDTSM forecasts numbers from historical metrics; Ollama is a language
model for text/reasoning. Path C uses Ollama for the **agent's brain**
(`| ai`), not for the dashboard forecast panels.

#### B3.6c. Optional — smoke-test `| ai` (Path C / Ollama)

**What B3.6c actually means:** this is an optional smoke test for Path C
— it checks that Splunk can call Ollama through the AI Toolkit for LLM
chat/reasoning. That is what the AegisOps agent uses when it decides
what action to take.

It has **nothing to do with fixing CDTSM**. Ollama is not a time-series
forecaster and cannot replace `| apply CDTSM`.

The simple `| ai prompt=prompt` query **without** a `provider=` argument
requires a **default LLM connection** in AITK. If you see
`No default LLM configuration found`, that is expected on Path B when you
have not configured Ollama in AITK yet.

Path C users who want SPL `| ai` (Ollama via AITK) should follow
[`docs/aitk-ollama.md`](docs/aitk-ollama.md): install Ollama, create an
**Ollama** connection in **Splunk AI Toolkit → Connection Management**,
then run:

```spl
| makeresults
| eval prompt="Reply with the single word pong."
| ai prompt=prompt provider=ollama_local model=gpt-oss:20b
```

This validates the **LLM path** for Path C (AegisOps agent reasoning).
It does **not** fix or replace the CDTSM forecast panels — those still
require Splunk Cloud. See [`docs/cdtsm-forecast.md`](docs/cdtsm-forecast.md)
for how CDTSM and the LLM work together in the full agent loop on Cloud.

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

**Leaving the daemon running overnight is fine.** If Splunk restarts or
HEC goes unreachable, the daemon logs `self-metric emit failed` every
~15 seconds but keeps accepting ingest on ports 5140/5141. Check health
anytime with `Invoke-RestMethod http://127.0.0.1:7321/api/status`
(`online: true` = process healthy). After Splunk is back, verify HEC with
`cargo run --release --bin aegis-daemon -- --check-hec` (`HEC ping
accepted`), then re-run the spammer to refresh dashboard panels.

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
| CDTSM forecast lines (bottom two panels) | B3.6 AI Toolkit + **Splunk Cloud with CDTSM/SLIM** + ~15 min of selfmetric data. **Not available on local Enterprise Developer License** — panels error with tenant 404; other 9 panels still work. |

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
| **Ollama** (mandatory) | See [C0 Install Ollama](#c0-install-ollama-one-time) below |
| Splunk auth token (recommended — enables SPL observations in the agent) | Splunk Web → **Settings → Tokens → New Token** with `search` capability. Paste into `[splunk].token` in `agent/configs/aegis-ops.toml`. Does **not** make CDTSM work on local Enterprise — see note below. |
| Splunk HEC token | Already configured in Path B — reuse for `[audit]` in the agent config |
| **Splunk's cloud AI service** _(optional — skip for Path C)_ | Splunk's **paid/cloud** way to run the official hosted LLMs (`gpt-oss-20b`, etc.) instead of Ollama. **Not available** on local Enterprise or most free trials — same wall as CDTSM. You do **not** need this; Path C uses **Ollama** by default. Only relevant if a Splunk sales engineer provisions a Cloud stack for you. Details: [`docs/splunk-blocker.md`](docs/splunk-blocker.md) |

> **Splunk token vs CDTSM:** the auth token lets the **AegisOps agent**
> run SPL searches against your Splunk (top signatures, classifier
> counts, trends). CDTSM inside the agent is **off by default**
> (`cdtsm_enabled = false`) and still needs **Splunk Cloud with SLIM**
> — same 404 gate as the dashboard forecast panels. On local Enterprise,
> configure the token for SPL observations; leave CDTSM disabled.

### C0. Install Ollama (one-time)

1. Download and install from <https://ollama.com/download> (Windows
   installer — same as any other app).
2. Open **any PowerShell window** (repo root, Desktop, anywhere — Ollama
   is system-wide, not tied to this project folder).
3. Pull the model that fits your RAM:

```powershell
# Default — needs ~16 GB system RAM
ollama pull gpt-oss:20b

# Smaller machines — pick one:
ollama pull qwen2.5:3b    # ~3 GB active, good JSON (~6–8 GB RAM total)
ollama pull qwen2.5:7b    # ~5 GB active
ollama pull gemma2:2b     # ~2 GB active
```

4. Confirm Ollama is serving:

```powershell
ollama list
# should show the model you pulled

curl.exe http://127.0.0.1:11434/api/tags
# should return JSON listing models
```

5. **Create your agent config** (one-time — same idea as copying
   `aegis.example.toml` → `aegis.toml` in Path B). Run:

```powershell
cd c:\Users\chukw\Desktop\splunk\agent
Copy-Item configs\aegis-ops.example.toml configs\aegis-ops.toml
```

   Then open `agent/configs/aegis-ops.toml` and set `[llm.ollama].model`
   to match what you pulled in step 3 (check with `ollama list`):

```toml
[llm.ollama]
url   = "http://127.0.0.1:11434"
model = "qwen2.5:3b"    # must match `ollama list` exactly
```

   If you pulled `gpt-oss:20b`, leave the default. Splunk tokens in
   this file come later (C3 optional section) — only the model line
   matters for now.

Ollama runs as a background service after install — you do **not** need
a dedicated terminal for it. Only the **agent** and **gateways** need
their own terminals in C2–C3.

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

C2 replaces the single B4 daemon (and any C1 demo daemons) with two
regional gateways wired to Splunk HEC. **Splunk and the sidecar must be
running before you start the gateways.**

#### C2a. Pre-flight — confirm Splunk and sidecar are up

Run these from the **repo root** (`c:\Users\chukw\Desktop\splunk`):

```powershell
# 1 — Stop C1 demo daemons if you ran them (free ports 5140/5142/7321/7322)
Get-Process aegis-daemon -ErrorAction SilentlyContinue | Stop-Process -Force

# 2 — Splunk Web (expect StatusCode 200)
try {
  (Invoke-WebRequest -Uri http://localhost:8000 -UseBasicParsing -TimeoutSec 5).StatusCode
} catch { Write-Host "Splunk Web DOWN — start Splunk (step 2b below)" }

# 3 — HEC ingest (expect: HEC ping accepted)
cargo run --release --bin aegis-daemon -- --check-hec

# 4 — Sidecar (expect JSON with "status":"ok" or similar)
curl.exe http://127.0.0.1:8765/health
```

| Check | Good sign | If it fails |
|---|---|---|
| Splunk Web | `200` | **Start Splunk** (see below) |
| HEC | `HEC ping accepted` | Splunk stopped, or bad token in `configs/aegis.toml` |
| Sidecar | HTTP 200 from `/health` | **Start sidecar** (see below) |

**Start Splunk** (if Web check failed):

```powershell
& "C:\Program Files\Splunk\bin\splunk.exe" start
# wait ~30s, then open http://localhost:8000 in a browser
```

**Start sidecar** (if `/health` connection refused) — **Terminal 1**, leave open:

```powershell
cd sidecar
.\.venv\Scripts\Activate.ps1
python -m aegis_sidecar.server
# expect: Uvicorn running on http://127.0.0.1:8765
```

Also stop the old **single B4 daemon** if it is still running (`Ctrl+C`
in its terminal). C2 uses the same ports for us-east (5140 / 7321).

#### C2b. Create regional configs and start gateways

Copy the same HEC token from your working Path B config
([`configs/aegis.toml`](configs/aegis.toml)) into both regional files:

```powershell
Copy-Item configs\aegis.us-east.example.toml configs\aegis-us-east.toml
Copy-Item configs\aegis.eu-west.example.toml configs\aegis-eu-west.toml
# Edit both files: replace PUT-YOUR-HEC-TOKEN-HERE with your real HEC token
```

**Terminal 2** (us-east):

```powershell
cargo run --release --bin aegis-daemon -- --config configs\aegis-us-east.toml
```

**Terminal 4** (eu-west — new window):

```powershell
cargo run --release --bin aegis-daemon -- --config configs\aegis-eu-west.toml
```

Verify both:

```powershell
Invoke-RestMethod http://127.0.0.1:7321/api/status
Invoke-RestMethod http://127.0.0.1:7322/api/status
```

Send traffic (same as C1, but now events land in Splunk with regional
`host=us-east` / `host=eu-west`):

```powershell
python demo\log_spammer.py --target tcp://127.0.0.1:5140 --pattern crashloop --rate 50 --duration 10
python demo\log_spammer.py --target tcp://127.0.0.1:5142 --pattern routine --rate 200 --duration 10
```

### C3. Configure and run the AegisOps agent

**Prerequisites:** C0 step 5 (agent config) and C2 (both gateways running).
Ollama must be running with the model named in your config.

One-time Python install (skip if you already ran this):

```powershell
cd agent
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e .
```

**Agent config — verify, don't blindly re-copy.** You created
`agent/configs/aegis-ops.toml` in [C0 step 5](#c0-install-ollama-one-time)
when you set `[llm.ollama].model` (e.g. `qwen2.5:3b`). **Do not run
`Copy-Item` again** — that would overwrite your edits.

Confirm before running (**must be inside `agent/`** — the config is
not in the repo-root `configs/` folder):

```powershell
cd c:\Users\chukw\Desktop\splunk\agent

# File exists?
Test-Path configs\aegis-ops.toml

# Model matches what Ollama has? (compare the two outputs)
ollama list
Select-String -Path configs\aegis-ops.toml -Pattern '^model\s*='
```

From repo root, use `agent\configs\aegis-ops.toml` instead of
`configs\aegis-ops.toml` — that root `configs/` folder is for the
**daemon** (Path B/C2), not the agent.

If `aegis-ops.toml` is missing (you skipped C0 step 5), create it once:

```powershell
Copy-Item configs\aegis-ops.example.toml configs\aegis-ops.toml
# then edit [llm.ollama].model to match `ollama list`
```

#### C3a. Wire Splunk + audit (required for Path C)

There is **no blocker** — unlike CDTSM, Splunk REST queries and HEC
audit work on local Enterprise. The example config leaves `[splunk]` and
`[audit]` empty only so the agent can run in a **credential-free**
smoke test (C1-style). **Path C expects both filled in** so the agent
reads trends from Splunk and writes decisions to `index=aegis`.

Edit `agent/configs/aegis-ops.toml`:

**1. Splunk auth token** (REST API — lets agent run SPL searches):

Splunk Web → **Settings** → **Tokens** → **New Token** → name
`aegis-ops`, capability **`search`** → **Create** → copy token.

**2. HEC token** — reuse the same token already in
[`configs/aegis.toml`](../configs/aegis.toml) (`[hec].token` from Path B).

**3. Paste both into the config:**

```toml
[splunk]
url        = "https://localhost:8089"   # REST API — port 8089, not Web 8000
token      = "YOUR-SPLUNK-AUTH-TOKEN"    # from step 1
verify_tls = false

[audit]
hec_endpoint = "https://localhost:8088/services/collector/event"
hec_token    = "YOUR-HEC-TOKEN"          # same as configs/aegis.toml [hec].token
verify_tls   = false
```

Leave `cdtsm_enabled = false` — CDTSM still needs Splunk Cloud (same
404 as dashboard forecast panels). SPL observations and audit **do**
work locally.

#### C3b. Run the agent

```powershell
cd c:\Users\chukw\Desktop\splunk\agent
.\.venv\Scripts\Activate.ps1
aegis-ops run --config configs\aegis-ops.toml --once -v
```

The first tick takes ~10 seconds (Ollama loads the model). Expected
output **with Splunk + audit configured**:

```
INFO AegisOps starting: 2 gateway(s), policy=low_risk_auto, dry_run=False, llm=ollama, splunk=on, audit=on
INFO [us-east] decision=noop(-) conf=0.95 exec=auto      | gateway healthy, no actionable signal
INFO [eu-west] decision=noop(-) conf=0.95 exec=auto      | gateway healthy, no actionable signal
```

If you see `splunk=off, audit=off`, `[splunk].url` / `[audit].hec_token`
are still empty — complete C3a above.

**Dry-run only** (prompt debugging — skips actuation and HEC writes):

```powershell
aegis-ops run --config configs\aegis-ops.toml --dry-run --once -v
```

Verify audit trail in Splunk — **Search & Reporting**, time range *Last
24 hours*:

```spl
index=aegis sourcetype=aegis:agent
| table _time, gateway, decision.action, exec_mode, decision.confidence, decision.justification
| sort -_time
```

To switch from Ollama to Splunk Hosted Models (when SLIM access is
available), change `[llm].transport` to `"splunk_ai"`.

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
| PSC install fails (`Winsock 10054`, `Internal Server Error`, `Package is too large`) | PSC (~800 MB) must use **CLI**, not Splunk Web. Download the `.tgz` from [Splunkbase](https://splunkbase.splunk.com/app/2883), then while Splunk is **running**: `splunk install app path\to\file.tgz -update 1 -auth user:pass` → `splunk restart`. See [B3.6 App 1](#b36-install-splunk-ai-toolkit-required--powers-cdtsm-forecast-panels). |
| `install app` says `splunkd is unreachable` | Splunk is stopped — run `splunk start` first, wait for Splunk Web, then `install app`. |
| AITK web Install fails | AITK is only ~30 MB — retry **Apps → Find More Apps → Splunk AI Toolkit → Install**. If it keeps failing, download from [Splunkbase](https://splunkbase.splunk.com/app/2890) and use the same CLI `install app` path as PSC. |
| `Error in 'ai' command: User does not have permission` | Grant **`apply_ai_commander_command`** on your Splunk role: **Settings → Roles → admin → Capabilities**. See [B3.6a](#b36a-grant-ai-command-permission-one-time). |
| `Error in 'ai' command: No default LLM configuration found` | You have not configured a default LLM in AITK Connection Management. For Path C, set up Ollama — see [B3.6c](#b36c-optional--smoke-test-ai-path-c--ollama) and [`docs/aitk-ollama.md`](docs/aitk-ollama.md). Unrelated to CDTSM. |
| `CDTSM: Failed to retrieve tenant info: HTTP 404 Not Found` | **Expected on local Splunk Enterprise.** CDTSM is Splunk-Hosted (SLIM/Cloud only). Path B still works — 9 of 11 dashboard panels populate. See [B3.6b](#b36b-smoke-test-cdtsm-splunk-cloud-only) and [`docs/splunk-blocker.md`](docs/splunk-blocker.md). |
| Dashboard CDTSM panels show `Unknown search command 'apply'` | AI Toolkit not installed. Complete [B3.6](#b36-install-splunk-ai-toolkit-required--powers-cdtsm-forecast-panels), restart Splunk, re-open the dashboard. On Cloud, panels also need ~15 min of `aegis:selfmetric` data. |
| Daemon logs `self-metric emit failed` repeatedly | Splunk or HEC was temporarily unreachable (restart, sleep, network). The daemon keeps running — verify with `Invoke-RestMethod http://127.0.0.1:7321/api/status` (`online: true` = healthy). Fix Splunk/HEC: `cargo run --release --bin aegis-daemon -- --check-hec` should print `HEC ping accepted`. Re-run the spammer to refresh dashboard data. Optional: restart the daemon after Splunk is back up. |
| `npm install` in `ui/` is slow | First install is ~1 minute (82 packages). Subsequent runs are seconds. |
| Sidecar startup error: `ModuleNotFoundError: No module named 'aegis_sidecar'` | You're running `python server.py` directly. Use `pip install -e .` inside the `sidecar/` virtualenv, then `python -m aegis_sidecar.server`. |
| `pip install -e .` in `sidecar/` fails with `ConnectionResetError` while downloading `torch` | Transient network drop on the ~123 MB PyTorch wheel. Run `pip install torch --index-url https://download.pytorch.org/whl/cpu` first, then `pip install -e .` again — pip reuses cached packages. The `win_amd64` in the wheel name is correct for 64-bit Windows on Intel or AMD — it does not mean you need an AMD CPU. |
| Sidecar takes ~30 s on first `/classify` call | Lazy-loading the sentence-transformer model (~80 MB). Subsequent calls are sub-millisecond. |
| UI shows "UNREACHABLE" badge | Daemon isn't running, or its MCP/REST port differs from `7321`. Confirm the daemon log says `Control API at 127.0.0.1:7321/api/status`. |

---

## License

[MIT](LICENSE).
