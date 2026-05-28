# The decision card

Aegis's decision card replaces the old "Execute" button. Instead of
asking the engineer to pick a tool, Aegis answers a sharper question:
*what should you do next, and what fixed this last time?*

## What it carries

```json
{
  "kind": "decision_card",
  "decision_id": "1eb3cd4fb3eb3dae",
  "ts": 1779986242.097,
  "state": "red",
  "chain_id": "f57c17209f1ee3fd",
  "root_cause_service": "payment-api",
  "headline": "payment-api broke first. checkout followed 4s later. orders followed 8s later. Root cause: payment-api (100% confidence).",
  "suggested_next_step": "This looks 100% similar to a past incident (fixed in 2 min last time). Last time the cause was: \"DB pool exhausted under retry storm.\" The fix was: \"Increased pool to 32, retry interval to 30s.\" Start by verifying that.",
  "business_impact": "Handles all transaction processing.",
  "similar_incidents": [
    {
      "incident_id": "3fa2723b6d0e8b64",
      "similarity": 1.0,
      "past_root_cause_service": "payment-api",
      "past_cause": "DB pool exhausted under retry storm.",
      "past_fix": "Increased pool to 32, retry interval to 30s.",
      "past_resolved_in_minutes": 2
    }
  ]
}
```

Three signals do the heavy lifting:

| Signal              | Source                                                         |
|---------------------|----------------------------------------------------------------|
| `state`             | Causal chain firing (red) / orange watch / green idle           |
| `headline`          | Causal chain's temporal attribution, rendered as one sentence   |
| `suggested_next_step` | Best matching resolved past incident → falls back to first-time nudge |
| `business_impact`   | `[services]` block in `aegis.toml` — operator-curated text     |
| `similar_incidents` | Top-N matches from incident memory, each with cause + fix when set |

## Why no "execute" button

Splunk's own buyer's guide and the hackathon judging brief both ask for
the same thing: *a human gets better visibility and a clearer next move,
not a robot pressing buttons in production.*

The card therefore offers three actions:

* **`I'm on it`** — POSTs to `/api/decision/ack`. Logged for audit; no
  side effects on production.
* **`Show me more past incidents`** — opens the incident memory panel
  so the engineer can browse the long tail of past chains.
* **`This looks different`** — flags the card for review. Useful when
  Aegis's similarity engine misfires; the operator's feedback is logged
  so we can tune thresholds without retraining anything.

None of those reach into a customer's services. The bounded-window tools
that do (`diagnostic`, `override`) are tucked into the Advanced section
of the UI and the MCP `tool_router` — they're available for engineers
who want them, never the default path.

## State machine

```text
  ┌────────┐   chain fires  ┌────────┐
  │ GREEN  │ ────────────▶ │  RED   │
  │ (idle) │                │        │
  └────────┘                └───┬────┘
       ▲                        │
       │  idle_to_green_secs    │
       │  of quiet              │
       └──────── (auto) ────────┘

  Future work: ORANGE
  — surfaced when a single service is misbehaving (volume spike,
  rising signature velocity) but no multi-service chain has fired
  yet. The state machine has the variant today; the green→orange
  transition will land alongside CDTSM-trend integration in the next
  iteration.
```

`Control::set_state(…)` is the single mutation surface; the snapshot is
emitted on every `aegis:selfmetric` so dashboards colour their badge
correctly.

## How `suggested_next_step` is chosen

```rust
// gateway/aegis-core/src/decision.rs :: suggest_next_step
fn suggest_next_step(root, links, similar) -> String {
    let best_resolved = similar.iter()
        .find(|m| m.past_cause.is_some() && m.past_fix.is_some());

    if let Some(best) = best_resolved {
        // "Last time the cause was X. The fix was Y. Start by verifying that."
        …
    }

    if let Some(best) = similar.first() {
        // Past incident matched but no resolution recorded yet.
        // Nudge the engineer to write one when they fix this.
        …
    }

    // Brand-new shape: tell them so, suggest where to look, ask them to
    // record a resolution card when they fix it.
    …
}
```

The function prefers the highest-similarity match with a recorded fix
over a higher-similarity match without one. That ordering is what makes
Aegis useful at 2 a.m. — an unresolved past incident isn't useful;
a resolved one with a known fix is the whole point.

## Where the card shows up

* **React UI**: hero panel above the fold, with the three buttons.
* **Splunk dashboard**: "Decision cards" table, time-sorted, last 24 h.
* **MCP**: `latest_decision` tool returns the same JSON.
* **REST**: `GET /api/decision`.
* **AegisOps agent**: pulled into every reasoner tick so the LLM is
  *grounded* in the gateway's vetted analysis, not asked to re-derive it.

## Resolution card

```json
POST /api/incidents/3fa2723b6d0e8b64/resolve
Content-Type: application/json

{
  "cause": "Database connection pool exhausted under retry storm.",
  "fix":   "Increased pool size from 8 to 32 and reduced retry interval to 30s."
}
```

Two sentences. That's the entire form. No mandatory templates. The
resolution gets stamped with `resolved_at` + `resolved_in_minutes` (delta
from the fingerprint's original timestamp), and the next similar chain
gets the engineer's words verbatim on its decision card.
