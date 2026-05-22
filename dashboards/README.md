# Aegis Splunk dashboard

`aegis.json` is a Splunk Dashboard Studio definition that visualises every
sourcetype the gateway emits.

## Install

1. In Splunk Web, go to **Dashboards → Create New Dashboard**.
2. Choose **Dashboard Studio**.
3. Click the source-editor icon (`{ }`) and paste the contents of
   `aegis.json` over the placeholder, or use **Import** if your Splunk
   version offers it.
4. Save. Set the time range to *Last 1 hour* so the panels populate.

## Panels

| Panel                          | What it tells you                                                                 |
|--------------------------------|------------------------------------------------------------------------------------|
| Dedup Savings                  | Live `dedup_savings_pct` from the gateway's self-metrics                          |
| Lines Ingested / Events Forwarded | Raw lines received vs Splunk events emitted — the FinOps headline                 |
| Queue Depth                    | How many events are buffered locally waiting for HEC                              |
| Ingest vs Forwarded            | Timechart proving the gateway holds the bottom line flat while ingest spikes      |
| Top suppressed signatures      | The biggest savers — collapse counts, average window, sample line                 |
| AI classifier verdict          | Pie of suppressed events by `anomaly` / `routine` / `unknown`                     |
| Classifier strategy used       | Whether the call went to the hosted model, embeddings, or the keyword fallback    |
| First-occurrence events        | Rate of *new* signatures — spikes here usually mean a deploy or incident          |

## Sourcetypes consumed

All panels read from `index=aegis` (configurable in `configs/aegis.toml`)
with the following sourcetypes:

* `aegis:selfmetric` — Self-emitted by the gateway every ~15s. Holds
  `events_in`, `events_out`, `dedup_savings_pct`, `queue_depth`,
  `online`, `unique_signatures`.
* `aegis:raw` — First-occurrence raw lines. `kind=first_occurrence`.
* `aegis:metric` — Dedup-collapsed metric events. Carries
  `signature`, `count`, `window_secs`, `sample`, and (when the AI
  sidecar is enabled) `classification.label` / `classification.confidence`
  / `classification.strategy`.
* `aegis:diagnostic` — Startup pings from `aegis-daemon --check-hec`.

## SPL crib sheet (for the demo video)

```spl
# Dedup ratio over the last hour, minute resolution
index=aegis sourcetype=aegis:selfmetric
| timechart span=1m latest(dedup_savings_pct) AS pct

# Top 10 services suppressed by signature
index=aegis sourcetype=aegis:metric
| stats sum(count) AS suppressed by signature
| sort - suppressed
| head 10

# Money-shot single-stat: total raw lines collapsed
index=aegis sourcetype=aegis:metric
| stats sum(count) AS lines_suppressed

# Anomalies the AI flagged that we didn't drop
index=aegis sourcetype=aegis:metric "classification.label"=anomaly
| stats sum(count) AS count by signature
| sort - count
```
