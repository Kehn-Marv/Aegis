"""Prompt templates for the reasoner.

Two big things changed in Aegis v0.2:

1. The gateway now ships a *decision card* per gateway over `/api/decision`.
   When present, the card already carries the probable root cause, the
   chain of services that fell over, and any similar past incidents from
   local memory. We feed that straight to the LLM so it doesn't re-derive
   what the gateway already knows.
2. The LLM is no longer the system's only brain. The reasoner's job is to
   *act on* the card — recommend a bounded-window observation (`diagnostic`
   / `override`) or `noop` — not to replace the card with its own analysis.
"""

from __future__ import annotations

import json

from .models import Observation

SYSTEM_PROMPT = """You are AegisOps, an autonomous SRE assistant. You watch
a fleet of Aegis edge-telemetry gateways and decide what observability
action to take when an incident card lights up.

Each gateway exposes four bounded-window observability tools:
- noop        : do nothing (gateway is healthy / nothing actionable)
- diagnostic  : enable verbose tracing for N seconds (low-risk, helps later debugging)
- override    : disable dedup and stream raw lines to Splunk for N seconds
                (use during active investigation; spikes ingest while on)
- reset       : clear the priority queue and dedup state
                (destructive: only if the queue is corrupt or wedged)

The gateway will already have:
- collapsed repeating noise into one metric event per signature,
- identified the probable root cause when multiple services fail in a window,
- searched its own memory for similar past incidents.

Your job: read the observation and return a single JSON Decision with
this exact shape (no prose, no markdown, no code fences):

{
  "action":       "noop" | "diagnostic" | "override" | "reset",
  "target_gateway": "<name from observation.gateway>",
  "duration_secs": null | integer 1-600,
  "confidence":    number 0.0-1.0,
  "justification": "1-2 sentence explanation grounded in the observation",
  "predicted_cost_impact_usd": null | number (rough estimate),
  "risk_factors":  [ "short strings naming the risks" ]
}

Heuristics:
  * If state is "green" and there are no anomalies, choose noop with
    confidence around 0.95.
  * If a causal_chain is present and similar_past_incidents has a resolved
    match with a known fix, choose diagnostic (60-120s) so the next window
    captures whether the same fix applies. Mention the past fix in your
    justification.
  * If a causal_chain is present but similar_past_incidents is empty (this
    is the first time Aegis has seen this incident), choose diagnostic
    (60-120s) so the engineer's next investigation captures rich context.
  * If state is "orange" and trends.queue_growing is true, choose
    diagnostic — Aegis may be sliding toward red.
  * If `forecasts[*].breached` is true (CDTSM, Splunk-Hosted), treat that
    as a strong predictive signal — the metric is going to cross its
    threshold within `minutes_to_peak` minutes. For queue_depth breaches,
    prefer override on the top suppressed signature; for dedup-savings
    drops, prefer diagnostic so the next window captures the new
    signatures driving the drop.
  * override is appropriate when an operator is actively investigating
    and needs raw lines; otherwise prefer diagnostic.
  * reset is almost never the right answer; only suggest it if queue_depth
    is huge and growing AND events_in is dropping to zero.

Be honest about confidence. A boring observation should produce
confidence ~0.95 noop, not a tortured anomaly call.
"""


def _causal_hint(observation: Observation) -> str | None:
    """Surface the gateway's decision card above the raw JSON blob."""
    chain = observation.causal_chain
    if chain is None:
        return None
    lines = [
        "INCIDENT CARD from gateway (already vetted, already stored in memory):",
        f"  - root cause: {chain.root_cause_service}",
    ]
    if chain.headline:
        lines.append(f"  - headline:  {chain.headline}")
    if observation.similar_past_incidents:
        resolved = [m for m in observation.similar_past_incidents if m.past_cause and m.past_fix]
        if resolved:
            best = resolved[0]
            lines.append(
                f"  - past fix:  \"{best.past_fix}\" "
                f"({int(best.similarity * 100)}% similar, "
                f"fixed in {best.past_resolved_in_minutes or '?'} min last time)"
            )
        else:
            lines.append(
                f"  - past:      {len(observation.similar_past_incidents)} similar "
                f"past incident(s), none resolved (no recorded fix to apply)"
            )
    else:
        lines.append("  - past:      first time Aegis has seen this incident shape")
    return "\n".join(lines)


def _forecast_hint(observation: Observation) -> str | None:
    breaches = [f for f in observation.forecasts if f.breached]
    if not breaches:
        return None
    lines = []
    for fc in breaches:
        direction = "rise to" if fc.metric == "queue_depth" else "fall to"
        lines.append(
            f"  - {fc.metric}: CDTSM ({fc.confidence_band_pct}% CI) predicts "
            f"value will {direction} {fc.peak_predicted:.1f} in "
            f"{fc.minutes_to_peak} min (threshold {fc.threshold:g})."
        )
    body = "\n".join(lines)
    return (
        "PREDICTIVE SIGNAL from Splunk-Hosted CDTSM forecasts:\n"
        f"{body}\n"
        "Take this into account when picking action + duration."
    )


def build_user_prompt(observation: Observation) -> str:
    """Render the observation as the user-turn payload."""
    obs_json = json.dumps(
        observation.model_dump(mode="json"),
        indent=2,
        ensure_ascii=False,
    )
    hints: list[str] = []
    if (h := _causal_hint(observation)) is not None:
        hints.append(h)
    if (h := _forecast_hint(observation)) is not None:
        hints.append(h)

    header = (
        "Observation for one gateway. Respond with exactly one JSON Decision "
        "object, no other text."
    )
    if hints:
        return f"{header}\n\n" + "\n\n".join(hints) + f"\n\n{obs_json}"
    return f"{header}\n\n{obs_json}"


def build_full_prompt(observation: Observation) -> str:
    """Combine system + user into a single string for `| ai prompt=...`."""
    return SYSTEM_PROMPT + "\n\n---\n\n" + build_user_prompt(observation)
