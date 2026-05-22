# Splunk AI Assistant 2.0 — integration notes

> How Aegis complements Splunk AI Assistant 2.0 (SAIA) in the
> Agentic Ops Hackathon context.

## What SAIA is

Splunk AI Assistant 2.0 is a **conversational agent inside Splunk Web**.
Operators ask natural-language questions; SAIA searches indexed data,
explains patterns, and suggests next steps using Splunk Hosted Models.

It is excellent for **human-in-the-loop investigation** after data is
already in Splunk.

## What SAIA is not (today)

As of the 2026 hackathon window, SAIA does **not** expose a documented
programmatic REST API for external agents to:

* invoke SAIA's reasoning loop from outside Splunk Web,
* push decisions back to edge infrastructure,
* or subscribe to SAIA conversation state.

That gap is intentional product scope, not a limitation of Aegis.

## How Aegis fills the gap

| Capability | SAIA 2.0 | Aegis + AegisOps Agent |
|------------|----------|------------------------|
| Where it runs | Splunk Web UI | Edge gateway + autonomous Python agent |
| When it acts | Operator asks a question | Continuous observe → reason → act loop |
| Data it sees | Indexed Splunk events | Live gateway state **and** Splunk SPL signals |
| Actuation | Recommendations in chat | REST/MCP commands on running gateways |
| Audit trail | Chat history in Splunk | `sourcetype=aegis:agent` HEC events |

Aegis is **not a replacement** for SAIA. It is the **edge actuation
layer** SAIA cannot reach today.

## Recommended demo flow (both together)

1. **AegisOps Agent** detects rising anomaly velocity on `us-east`,
   auto-enables `diagnostic` for 60 s, logs the decision to
   `sourcetype=aegis:agent`.
2. Operator opens **Splunk AI Assistant 2.0** and asks:
   *"What did the Aegis agent decide in the last 5 minutes on us-east,
   and which signatures drove it?"*
3. SAIA searches `index=aegis sourcetype=aegis:agent` and explains the
   agent's reasoning in plain language.
4. Operator approves a high-risk `override` recommendation the agent
   logged (policy mode `low_risk_auto` does not auto-execute overrides).

This demonstrates **two agentic surfaces on one platform**:

* an autonomous edge agent (AegisOps), and
* a human-facing copilot (SAIA),

both grounded in the same Splunk data.

## SPL starter queries for SAIA

```spl
index=aegis sourcetype=aegis:agent
| table _time, gateway, decision.action, exec_mode, decision.confidence, decision.justification
| sort -_time
```

```spl
index=aegis sourcetype=aegis:metric host=us-east
| stats sum(count) AS suppressed by "classification.label"
| sort -suppressed
```

## Future integration (if Splunk ships an API)

If Splunk exposes a programmatic SAIA endpoint, AegisOps could add an
optional `saia.review_decision` step: after the hosted model proposes an
`override`, the agent would ask SAIA to sanity-check the justification
before actuation. The hook point is `agent/aegis_ops/reasoner.py` —
no gateway changes required.

Until then, the documented integration path above is the authentic
best-effort approach for the hackathon.
