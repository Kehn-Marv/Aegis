# Incident memory

Aegis's institutional memory: every causal chain becomes a fingerprint in
a local SQLite store. When a new chain happens, Aegis searches its memory
in sub-millisecond time and surfaces the past cause + fix on the new
decision card.

## What's in a fingerprint

```rust
// gateway/aegis-core/src/incident_memory.rs
pub struct Fingerprint {
    pub id: String,                       // short_uuid()
    pub chain_id: String,                 // ties back to the originating CausalChain
    pub ts: f64,                          // when the chain happened
    pub root_cause_service: String,
    pub services: Vec<String>,            // every service involved, in order
    pub signatures: Vec<String>,          // every signature involved
    pub chain: Vec<CausalLink>,           // full reconstruction
    pub cause: Option<String>,            // filled by the engineer on resolution
    pub fix: Option<String>,
    pub resolved_at: Option<f64>,
    pub resolved_in_minutes: Option<i64>,
}
```

Everything is stored in one SQLite table with three indexed columns
(`ts`, `chain_id`, plus the primary key). Service/signature sets are
JSON arrays inside their own columns so we can deserialise once per
similarity scan.

## How similarity works

When a fresh chain arrives, the store computes a score against every
stored fingerprint and returns the top matches over `min_similarity`:

```text
score = 0.50 * Jaccard(signatures)        # same error patterns?
      + 0.30 * Jaccard(services)          # same services involved?
      + 0.15 * LCS-ratio(service order)   # same temporal order?
      + 0.05 * (root_service matches)     # bonus
```

Weights live at the top of `incident_memory.rs::similarity()` so
operators can tune the formula without leaving the file.

* **Jaccard** is intersection / union of two sets — fast, intuitive,
  bounded [0, 1].
* **LCS ratio** is the longest common subsequence length divided by the
  longer chain length — captures temporal order similarity.

No ML model is involved. No embeddings, no vector DB, no network. The
store handles 10,000 incidents in well under one millisecond on a laptop
because it's literally `SELECT * FROM incidents` followed by a
Rust-side scan over small JSON blobs.

## The flow end-to-end

```text
CausalChain fires
    ↓
Store::record_chain() — write a new fingerprint (no cause/fix yet)
    ↓
Store::search_similar() — score against all known fingerprints
    ↓
DecisionCard emitted with similar_incidents = top-N matches
    ↓
Engineer acts; clicks "I'm on it"; investigates; fixes
    ↓
Engineer fills in the 2-line resolution card
    ↓
Store::resolve() — attach cause + fix to the fingerprint
    ↓
Next time a similar chain happens, the decision card carries
the engineer's fix verbatim
```

## REST and MCP surface

| Operation       | REST                                              | MCP tool             |
|-----------------|---------------------------------------------------|-----------------------|
| List incidents  | `GET /api/incidents?limit=N`                      | `recent_incidents`    |
| Read one        | `GET /api/incidents/{id}`                          | _(read via list)_     |
| Resolve         | `POST /api/incidents/{id}/resolve` `{cause, fix}` | `resolve_incident`    |

Both the React UI and the AegisOps agent use these endpoints. Tom in
Cursor can call `resolve_incident` mid-investigation to record the fix
without leaving the chat.

## The "first time" edge case

What if the new chain has no matches in memory? The decision card's
`similar_incidents` array is empty, and `suggested_next_step` says:

> *Check payment-api health and recent deploys. Earliest signal:
> ERROR payment-api: db pool exhausted. **This is the first time Aegis
> has seen this incident shape; please record a resolution card when you
> fix it.***

This nudges the engineer to feed the memory store on the way out. The
nudge is unit-tested at
`decision::tests::engine_handles_first_time_incident_gracefully`.

## Storage characteristics

| Property      | Value                                                                |
|---------------|----------------------------------------------------------------------|
| Backend       | SQLite (bundled, no system dependency)                               |
| File          | `[memory].path` (default `data/aegis-incidents.sqlite`)              |
| Memory cost   | ~1 KB per fingerprint in memory while scanning                        |
| Disk cost     | ~2 KB per fingerprint on disk                                         |
| Search cost   | O(N) on `recent(2048)` — sub-ms at typical N                          |
| Backup        | Just copy the `.sqlite` file. WAL mode is on; copy while running OK.  |

If you want to share institutional memory across two regional gateways,
point both `[memory].path` entries at the same network filesystem path.
SQLite handles WAL-mode concurrent reads happily; writes still serialise.
For larger fleets, run one Aegis daemon per region and replicate
incident memory upstream into Splunk under
`sourcetype=aegis:incident` — the audit trail is complete there too.
