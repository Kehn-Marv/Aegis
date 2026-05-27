# AegisOps Agent

Autonomous Python agent that closes the loop between Aegis edge
gateways and Splunk. Polls each gateway, queries Splunk for recent
telemetry the gateway emitted (optionally via the official **Splunk
MCP Server** as a JSON-RPC client), reads **CDTSM forecasts** of its
own future, **reasons with one of three live LLM transports**, and
either acts autonomously (low-risk tools) or recommends an action for
an operator to approve.

Every decision — prompt, model response, action taken, result — is
optionally logged to Splunk under `sourcetype=aegis:agent`.

> **LLM transports at a glance.** Three transports, switchable by a
> single config flag — see the table below. Default is raw Ollama
> running `gpt-oss:20b` (the same model identifier published by
> Splunk Hosted Models). Switch to `aitk_ollama` for the `| ai` SPL
> command routed through Splunk's AI Toolkit Connection Management
> (see [`../docs/aitk-ollama.md`](../docs/aitk-ollama.md)), or to
> `splunk_ai` for true SLIM-backed Splunk-Hosted Models when
> provisioned (see [`../docs/splunk-blocker.md`](../docs/splunk-blocker.md)).

## Why this exists

Aegis's MCP server already lets a human (or Cursor / Claude Desktop)
poke the gateway. The agent is what makes it **agentic in the literal
sense**: an autonomous loop that detects, decides, and acts without a
human in the prompt loop.

## What the agent does each loop

```
for each gateway in config:                                  # multi-edge
    observation = {
        live status from gateway's REST /api/status,
        SPL-derived signals from Splunk if configured
          (top signatures, anomaly count, classifier breakdown,
           rolling trends, trajectory label)
    }
    decision = llm_transport.call(prompt)                    # Ollama or | ai
    if policy says auto-actuate:
        result = call_gateway_command(decision)
    else:
        result = "recommendation logged for operator"
    audit(prompt, decision, action, result) → optional HEC
```

## LLM transports

| Transport       | Status     | Requires                                                                | File                                                              |
|-----------------|------------|-------------------------------------------------------------------------|-------------------------------------------------------------------|
| `ollama`        | **Default**| Local Ollama running `gpt-oss:20b` (~16 GB RAM) or smaller fallback     | `aegis_ops/transports.py :: OllamaTransport`                       |
| `aitk_ollama`   | Live       | Splunk Enterprise + AI Toolkit + AITK Ollama LLM connection             | `aegis_ops/transports.py :: SplunkAITransport` (provider=ollama_local) |
| `splunk_ai`     | One-line   | Splunk Cloud SLIM access (gated on 14-day trial)                        | `aegis_ops/transports.py :: SplunkAITransport` (provider=splunk_hosted) |

All three produce the same JSON `Decision`. Switching is one line in
`configs/aegis-ops.toml`.

The Ollama transport also passes the `Decision` Pydantic JSON schema
to Ollama's `format` parameter, which **enforces the schema at decode
time** — so even a small model on a low-RAM machine can't emit
malformed JSON. Big reliability win for development.

## Policy (the safety rail)

By default the agent operates in **`low_risk_auto`** mode:

| Tool        | Default behaviour |
|-------------|-------------------|
| `diagnostic` | auto-executed |
| `status`     | auto-executed (read-only) |
| `noop`       | auto, just logs heartbeat |
| `override`   | **recommendation only** (logged for operator) |
| `reset`      | **recommendation only** (logged for operator) |

Policy is configurable in `configs/aegis-ops.toml`. Modes:
`read_only`, `low_risk_auto` (default), `full_auto`.

## Run (Plan B: zero Splunk credentials)

### 1. Install Ollama and pull the model

* Download Ollama: <https://ollama.com/download>
* Pull the right model for your machine's total RAM:

  | RAM (total system) | Model | Pull command | Disk | Active RAM |
  |---|---|---|---|---|
  | **16 GB+** (default — matches Splunk Hosted Models name) | `gpt-oss:20b` | `ollama pull gpt-oss:20b` | ~13 GB | ~16 GB |
  | 8–16 GB | `qwen2.5:7b` | `ollama pull qwen2.5:7b` | 4.5 GB | ~5 GB |
  | 6–8 GB | `qwen2.5:3b` | `ollama pull qwen2.5:3b` | 1.9 GB | ~3 GB |
  | 4–6 GB | `gemma2:2b` | `ollama pull gemma2:2b` | 1.6 GB | ~2 GB |
  | <4 GB | `qwen2.5:1.5b` | `ollama pull qwen2.5:1.5b` | 1.0 GB | ~1.5 GB |

* Verify: `ollama run gpt-oss:20b "say hello"` should reply (replace
  with the smaller model you pulled if you downshifted).

* If you pick a non-default model, set it in `configs/aegis-ops.toml`:

  ```toml
  [llm.ollama]
  model        = "qwen2.5:3b"   # or whichever you pulled
  timeout_secs = 600            # CPU Ollama: ~5 min per gateway; see ../Troubleshooting.md
  ```

### 2. Launch the agent

```powershell
cd agent
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e .
Copy-Item configs\aegis-ops.example.toml configs\aegis-ops.toml
# Edit configs\aegis-ops.toml — add Splunk + HEC tokens for Path C (see README C3a)
aegis-ops --config configs\aegis-ops.toml --once -v
```

`configs/aegis-ops.toml` is **gitignored** — only the example file is
tracked. Never commit tokens.

On **CPU-only** Ollama, expect **~4–5 minutes per gateway** for the
first reasoning call. Success looks like `conf=0.95`, not `conf=0.00`.
Use `--once` for smoke tests; continuous mode waits for each slow tick
to finish (see `[agent].loop_interval_secs` note in the example config).

You'll see one line per gateway:

```
[12:34:01] gateway=us-east  decision=diagnostic(60s)  conf=0.82  exec=auto    queue depth climbing
[12:34:01] gateway=eu-west  decision=noop             conf=0.95  exec=auto    all metrics nominal
```

### 3. Launch the two demo gateways first

In a separate terminal:

```powershell
.\demo\run-multi-edge.ps1
```

## Run (full stack: Ollama + Splunk observations + HEC audit)

Edit `configs/aegis-ops.toml`:

```toml
[splunk]
url        = "https://localhost:8089"
token      = "paste-search-token"   # Settings -> Tokens -> New Token
verify_tls = false                  # local Enterprise self-signed cert

[audit]
hec_endpoint = "https://localhost:8088/services/collector/event"
hec_token    = "paste-hec-token"
verify_tls   = false
```

The agent now observes `index=aegis` SPL signals AND ships every
decision to HEC. The LLM is still local Ollama — no SLIM API needed.

## Run (Plan A: native Splunk Hosted Models, when provisioned)

When a Splunk Cloud account with SLIM access is available:

```toml
[llm]
transport = "splunk_ai"

[llm.splunk_ai]
provider = "splunk_hosted"
model    = "gpt-oss-20b"
```

No code changes. Same prompt, same decision schema, same audit trail.

## Dry-run

```powershell
aegis-ops --config configs\aegis-ops.toml --dry-run
```

Actuator never calls any gateway and never writes to HEC. Useful for
iterating on the prompt without producing side effects.

## Audit trail in Splunk (when `[audit]` is configured)

Run this in **Splunk Web** (`http://localhost:8000`), not in a terminal:

1. Open **Search & Reporting**.
2. Paste into the search bar and click **Search** (time range *Last 24
   hours*, or *Last 15 minutes* if you just ran the agent):

```spl
index=aegis sourcetype=aegis:agent
| sort - _time
| table _time, gateway, decision.action, decision.confidence, exec_mode, decision.justification
```

Every decision the agent made — action, confidence, and justification —
is visible to the SRE team. See [README C3c](../README.md#c3c-verify-agent-decisions-in-splunk)
for expected output and empty-result fixes.

<!-- Tests are intentionally not committed to this repo (see .gitignore).
     Devs adding tests locally can install dev deps with:
       pip install -e ".[dev]"
     and run them with `python -m pytest`. -->

