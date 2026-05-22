# AegisOps Agent

Autonomous Python agent that closes the loop between the Aegis edge
gateways and Splunk. It runs as a **separate process** (deliberately
not embedded in the daemon), polls each gateway, queries Splunk for
recent telemetry the gateway emitted, reasons with a Splunk Hosted
Model about what to do next, and either acts autonomously (low-risk
tools) or recommends an action for an operator to approve (high-risk
tools like `override`/`reset`).

Every decision — prompt, model response, action taken, result — is
logged back to Splunk under `sourcetype=aegis:agent` for full
auditability.

## Why this exists

Aegis's MCP server already lets a human (or Cursor / Claude Desktop)
poke the gateway. The agent is what makes it **agentic in the literal
sense**: an autonomous loop that detects, decides, and acts without a
human in the prompt loop. It's the difference between *agentic-capable
infrastructure* (the gateway) and *an actual agent* (this package).

## What the agent does each loop

```
for each gateway in config:                                  # multi-edge
    observation = {
        live status from gateway's REST /api/status,
        SPL-derived signals from Splunk (top signatures,
        anomaly count, classifier breakdown, rolling trends)
    }
    decision = call_splunk_hosted_model(observation)         # | ai SPL
    if policy says auto-actuate:
        result = call_gateway_command(decision)
    else:
        result = "recommendation logged for operator"
    audit(prompt, decision, action, result) → Splunk HEC
```

## Reasoning model

The agent reasons with whichever model the AI Toolkit's Connection
Management has wired as the Splunk Hosted (SLIM API) provider's default
— typically `gpt-oss-20b` for cost/latency, optionally `gpt-oss-120b`
for harder triage. The prompt template lives in
[`aegis_ops/prompts.py`](aegis_ops/prompts.py) and is structured to
produce a strict JSON `Decision` object the actuator can validate.

## Policy (the safety rail)

By default the agent operates in **`low_risk_auto`** mode:

| Tool        | Default behaviour |
|-------------|-------------------|
| `diagnostic` | auto-executed |
| `status`     | auto-executed (read-only) |
| `noop`       | auto, just logs heartbeat |
| `override`   | **recommendation only** (logged for operator) |
| `reset`      | **recommendation only** (logged for operator) |

Policy is configurable in `configs/aegis-ops.toml`. You can flip to
`full_auto` or `read_only` per environment.

## Run

```powershell
cd agent
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e .
# Edit configs/aegis-ops.toml (Splunk URL, token, gateway URLs)
aegis-ops run --config configs\aegis-ops.toml
```

You'll see one line per loop iteration:

```
[12:34:01] gateway=us-east  decision=diagnostic(60s)  conf=0.82  exec=auto    "queue depth climbing, recommend deeper trace"
[12:34:01] gateway=eu-west  decision=noop             conf=0.95  exec=auto    "all metrics nominal"
[12:34:06] gateway=us-east  decision=override(30s)    conf=0.91  exec=recommend "anomaly cluster forming, suggest raw passthrough"
```

## Dry-run

```powershell
aegis-ops run --config configs\aegis-ops.toml --dry-run
```

Same loop, but the actuator never calls any gateway and never writes
to HEC. Useful for iterating on the prompt without producing audit noise.

## Multi-edge demo (credential-free)

Launch two gateways without Splunk:

```powershell
# From repository root
.\demo\run-multi-edge.ps1
```

Then dry-run the agent against both:

```powershell
cd agent
pip install -e .
aegis-ops run --config configs\aegis-ops.example.toml --dry-run --once -v
```

The example config already lists `us-east` (:7321) and `eu-west` (:7322).
Splunk SPL calls will fail gracefully until you paste credentials — the
agent still observes live gateway status and produces noop decisions.

## Audit trail in Splunk

```spl
index=aegis sourcetype=aegis:agent
| table _time, gateway, decision.action, decision.confidence, exec, decision.justification
| sort -_time
```

Every decision the agent ever made, including the model's reasoning,
end-to-end visible to the SRE team.
