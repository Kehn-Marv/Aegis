# Causal chain detection

When several services start failing in the same window, Aegis attributes
the root cause to **whichever service broke earliest**. Everything that
broke after it is collateral damage.

## The signal

The dedup stage already emits one `FirstOccurrence` event the first time
a new signature is seen  -  that's the moment of "this is new". The causal
engine watches every **anomalous** `FirstOccurrence` (errors, warnings, and
stack-trace continuations), tags it with the service that produced it, and
keeps a small ring buffer per service. Routine `INFO`/`DEBUG` first-sightings
are ignored: a healthy service simply being *seen* for the first time is not
evidence of an incident, so a busy multi-service fleet never looks like a
perpetual outage.

On each event, the engine asks: *in the last `window_secs`, how many
distinct services first-fired a new signature?* If the count crosses
`min_services`, the engine produces a `CausalChain` event whose
`chain` field lists every involved service in temporal order.

## Worked example

Imagine the demo `cascade` pattern (run via `python demo/log_spammer.py
--pattern cascade`):

```text
t=0s    payment-api    "db connection pool exhausted"
t=4s    checkout       "payment-api unreachable"
t=8s    orders         "cannot create order, checkout never completed"
```

With `min_services=3` and `window_secs=30`, the engine fires:

```json
{
  "kind": "causal_chain",
  "chain_id": "4a9d4f9e…",
  "root_cause_service": "payment-api",
  "confidence": 1.0,
  "chain": [
    { "service": "payment-api", "ts_offset_secs": 0.0, "signature": "…", "sample": "ERROR payment-api: db pool exhausted" },
    { "service": "checkout",    "ts_offset_secs": 4.0, "signature": "…", "sample": "ERROR checkout: payment-api unreachable" },
    { "service": "orders",      "ts_offset_secs": 8.0, "signature": "…", "sample": "ERROR orders: cannot create order" }
  ]
}
```

Output isn't 200 alerts. It's *one* attribution sentence.

## Confidence

The engine scores confidence from the spread between the earliest and
latest links. A clean spread of several seconds gives ≥ 0.95; a tied set
of failures (everything in the same instant) bottoms out around 0.65
because the temporal order is genuinely ambiguous.

```rust
// gateway/aegis-core/src/causal.rs
fn score_confidence(chain: &[CausalLink]) -> f32 {
    if chain.len() < 2 { return 0.5; }
    let max_offset = chain.iter().map(|l| l.ts_offset_secs).fold(0.0, f64::max);
    let base = 0.65 + 0.35 * (max_offset / 5.0).min(1.0);
    base.clamp(0.0, 1.0) as f32
}
```

## Edge cases the detector handles

| Scenario                                              | Behaviour                                                            |
|-------------------------------------------------------|----------------------------------------------------------------------|
| Routine `INFO`/`DEBUG` first-sightings                | Ignored. Only anomalous (WARN/ERROR) lines seed a chain.            |
| One service spamming many signatures                  | Does **not** fire. We require distinct *services*, not signatures.   |
| Two services failing simultaneously                   | Fires, but confidence is low (~0.65). Operator sees the ambiguity.  |
| Long-running outage (hours)                           | Fires once. `cooldown_secs` suppresses re-emission for the same root.|
| Stack trace continuation lines (`  at …`)             | Inherit the parent line's service via per-source last-service cache. |
| Service mentioned but never first-fires a new signature | Not included in the chain (we only see new patterns).               |
| Per-service event storm                               | Bounded by `per_service_buffer` (default 16 entries).                |

## Tuning

```toml
[causal]
window_secs   = 60     # window for grouping
min_services  = 3      # how many distinct services trigger a chain
cooldown_secs = 300    # suppress re-emission for this many seconds
```

Lower `min_services` to 2 in environments where two-service correlations
*are* meaningful (small fleets, edge clusters). Raise `cooldown_secs`
during long incidents you don't want re-alerting on.

## How service names are extracted

`service::extract_full` tries the following, in order:

1. **Config hint**  -  an explicit `source_to_service` entry pins a known
   source (e.g. `"tcp://10.0.4.12:5140" = "us-east-payment"`).
2. **Continuation inheritance**  -  stack-trace frames and `caused by:`
   lines reuse the most recent service for the same source.
3. **JSON `service` field**  -  `{"service":"payment-api", …}`.
4. **`svc=name` / `service=name` key-value**  -  common in
   structured-text formats.
5. **`LEVEL service:` prefix**  -  `ERROR payment-api: connection refused`.
6. **Bracket prefix**  -  `[payment-api] doing work` (level words rejected).
7. **Fallback**  -  the ingest source string (`tcp://host:port`). Never
   empty.

This is unit-tested at `gateway/aegis-core/src/service.rs`.
