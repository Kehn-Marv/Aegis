# AegisOps Agent

Autonomous Python agent that closes the loop between Aegis edge
gateways and Splunk. Polls each gateway, queries Splunk for recent
telemetry the gateway emitted, **reasons with a local Ollama LLM
(default: `qwen2.5:3b`, ~3 GB RAM) or a Splunk Hosted Model
(hibernated)** about what to do next, and either acts autonomously
(low-risk tools) or recommends an action for an operator to approve.

Every decision — prompt, model response, action taken, result — is
optionally logged to Splunk under `sourcetype=aegis:agent`.

> **About the LLM transport.** This agent originally targeted Splunk
> Hosted Models via the AI Toolkit `| ai` SPL command. The 14-day
> Splunk Cloud trial does not provision the SLIM API, so we ship
> Ollama as the default. The Splunk `| ai` integration is
> **preserved, tested, and one config flag away** — see
> [`../docs/splunk-blocker.md`](../docs/splunk-blocker.md).

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

| Transport     | Status     | Requires                                    | File                                          |
|---------------|------------|---------------------------------------------|-----------------------------------------------|
| `ollama`      | **Default**| Local Ollama running `qwen2.5:3b` (~3 GB)   | `aegis_ops/transports.py :: OllamaTransport`  |
| `splunk_ai`   | Hibernated | Splunk SLIM API access (trial-gated)        | `aegis_ops/transports.py :: SplunkAITransport` |

Both produce the same JSON `Decision`. Switching is one line in
`configs/aegis-ops.toml`.

The Ollama transport also passes the `Decision` Pydantic JSON schema
to Ollama's `format` parameter, which **enforces the schema at decode
time** — so even a 3B model can't emit malformed JSON. Big reliability
win on small machines.

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
  | **6–8 GB** (default) | `qwen2.5:3b` | `ollama pull qwen2.5:3b` | 1.9 GB | ~3 GB |
  | 4–6 GB | `gemma2:2b` | `ollama pull gemma2:2b` | 1.6 GB | ~2 GB |
  | <4 GB | `qwen2.5:1.5b` | `ollama pull qwen2.5:1.5b` | 1.0 GB | ~1.5 GB |
  | 16 GB+ (best quality) | `qwen2.5:7b` | `ollama pull qwen2.5:7b` | 4.5 GB | ~5 GB |

* Verify: `ollama run qwen2.5:3b "say hello"` should reply.

* If you pick a non-default model, set it in `configs/aegis-ops.toml`:

  ```toml
  [llm.ollama]
  model = "gemma2:2b"   # or whichever you pulled
  ```

### 2. Launch the agent

```powershell
cd agent
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e .
Copy-Item configs\aegis-ops.example.toml configs\aegis-ops.toml
# (no edits needed — defaults run pure-Ollama against two local gateways)
aegis-ops run --config configs\aegis-ops.toml --once -v
```

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
url   = "https://prd-p-XXXXX.splunkcloud.com"
token = "paste-search-token"   # Settings -> Tokens -> New Token

[audit]
hec_endpoint = "https://localhost:8088/services/collector/event"
hec_token    = "paste-hec-token"
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
aegis-ops run --config configs\aegis-ops.toml --dry-run
```

Actuator never calls any gateway and never writes to HEC. Useful for
iterating on the prompt without producing side effects.

## Audit trail in Splunk (when `[audit]` is configured)

```spl
index=aegis sourcetype=aegis:agent
| table _time, gateway, decision.action, decision.confidence, exec_mode, decision.justification
| sort -_time
```

Every decision the agent ever made, including the model's reasoning,
end-to-end visible to the SRE team.

<!-- Tests are intentionally not committed to this repo (see .gitignore).
     Devs adding tests locally can install dev deps with:
       pip install -e ".[dev]"
     and run them with `python -m pytest`. -->

