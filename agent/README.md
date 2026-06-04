# AegisOps Agent

Autonomous Python agent that observes Aegis edge gateways, reasons with
a local LLM, and recommends or actuates bounded-window observability
actions. Every decision is shipped to Splunk under
`sourcetype=aegis:agent` for full audit.

The agent reads the gateway's **decision card** directly  -  Aegis has
already done the causal attribution and looked up similar past
incidents, so the LLM's job is to *act on* that card, not re-derive it.

## What the agent does each tick

```text
for each gateway in config:
    observation = REST /api/status + /api/decision + (optional) SPL signals
    decision    = LLM(observation)                       # via Ollama / | ai
    if policy says auto-actuate:
        gateway.POST /api/command                         # only low-risk by default
    audit(prompt, decision, action, result) → HEC sourcetype=aegis:agent
```

## LLM transports

| Transport       | Status     | Requires                                                              |
|-----------------|------------|------------------------------------------------------------------------|
| `ollama`        | **Default**| Local Ollama running `gpt-oss:20b` (~16 GB RAM) or a smaller fallback  |
| `aitk_ollama`   | Live       | Splunk Enterprise + AI Toolkit + AITK Ollama LLM connection           |
| `splunk_ai`     | One-line   | Splunk Cloud SLIM access (gated on the 14-day trial)                  |

All three produce the same JSON `Decision`. Switching is one line in
`configs/aegis-ops.toml`.

The Ollama transport passes the `Decision` Pydantic JSON schema to
Ollama's `format` parameter, which **enforces the schema at decode
time**  -  even a small model can't emit malformed JSON.

## Policy modes

| Mode             | Behaviour                                                       |
|------------------|------------------------------------------------------------------|
| `read_only`      | Every decision becomes a recommendation; nothing actuates       |
| `low_risk_auto`  | Default. `diagnostic`/`noop` auto-execute; `override`/`reset` recommend |
| `full_auto`      | Everything auto-executes (use carefully)                        |

Plus a per-tool cooldown that prevents the agent from firing the same
tool twice within `cooldown_secs` for the same gateway.

## Quick start

```powershell
# 1. Install Ollama and pull a model:
#    https://ollama.com/download
ollama pull qwen2.5:3b      # ~3 GB RAM, good JSON quality
# or for 16 GB+ machines:
ollama pull gpt-oss:20b     # matches Splunk Hosted Models identifier

# 2. Install the agent
cd agent
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e .
Copy-Item configs\aegis-ops.example.toml configs\aegis-ops.toml

# 3. Edit configs\aegis-ops.toml:
#    - [llm.ollama].model = "qwen2.5:3b"  (must match what you pulled)
#    - [splunk] url + token  (optional, enables SPL observations)
#    - [audit] hec_endpoint + hec_token  (optional, enables HEC audit)

# 4. Make sure at least one Aegis gateway is running
#    (configs/aegis.demo.toml works without Splunk)
.\..\target\debug\aegis-daemon.exe --config ..\configs\aegis.demo.toml

# 5. Run the agent
aegis-ops --config configs\aegis-ops.toml --once -v   # Run the agent once, then exits (smoke test)
aegis-ops --config configs\aegis-ops.toml -v            # Runs continuously until you press Ctrl+C
```

On **CPU-only Ollama** the first reasoning call takes ~4–5 minutes per
gateway (the model has to load). Subsequent calls are seconds. Set
`[llm.ollama].timeout_secs = 600` and warm the model with
`ollama run qwen2.5:3b "reply pong"` before the agent run.

Success looks like:

```text
INFO AegisOps starting: 1 gateway(s), policy=low_risk_auto, dry_run=False, llm=ollama, splunk=off, audit=off
INFO [us-east] decision=noop(-) conf=0.95 exec=auto  | gateway healthy, no actionable signal
```

`conf=0.95` means the model returned a real decision. `conf=0.00` means
the model returned nothing parseable  -  usually a timeout.

## Audit trail

When `[audit]` is configured, every decision lands in Splunk:

```spl
index=aegis sourcetype=aegis:agent
| sort - _time
| table _time, gateway, decision.action, decision.confidence,
        exec_mode, decision.justification
```

Each row carries the full prompt + raw model response, so any
skeptical operator can read exactly how the agent decided.

## Dry-run mode

```powershell
aegis-ops --config configs\aegis-ops.toml --dry-run --once -v
```

Skips actuation and HEC writes. Useful when iterating on the prompt.

## How the agent uses Aegis's decision card

When the gateway has fired a `CausalChain`, its `/api/decision`
endpoint returns the same `DecisionCard` the UI sees. The agent
forwards that to the LLM as an `INCIDENT CARD` block above the raw
observation JSON:

```text
INCIDENT CARD from gateway (already vetted, already stored in memory):
  - root cause: payment-api
  - headline:  payment-api broke first. checkout followed 4s later. orders followed 8s later. Root cause: payment-api (100% confidence).
  - past fix:  "Increased pool to 32, retry interval to 30s" (100% similar, fixed in 2 min last time)

Observation for one gateway. Respond with exactly one JSON Decision object...
```

This is **load-bearing**: the LLM is grounded in the gateway's
deterministic analysis, not asked to re-derive it. The LLM's only
remaining job is to pick a bounded-window action (`diagnostic`,
`override`, `noop`, `reset`) based on policy.

See [`../docs/decision-card.md`](../docs/decision-card.md) for the full
decision-card shape.
