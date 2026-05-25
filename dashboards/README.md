# Aegis Splunk dashboard

`aegis.json` is a Splunk Dashboard Studio definition that visualises every
sourcetype the gateway emits.

## Install

Requires Path B [B3.5](../README.md#b35-install-the-ai-sidecar-required--powers-classifier-panels)
(sidecar) and [B3.6](../README.md#b36-install-splunk-ai-toolkit-required--powers-cdtsm-forecast-panels)
(AI Toolkit) before all 11 panels populate.

1. In Splunk Web (`http://localhost:8000`), open **Search & Reporting**
   → **Dashboards** → **Create New Dashboard**.
2. On the dialog: set **Dashboard Title** to `Aegis`, leave
   **Permissions** as *Private*, choose **Dashboard Studio**, select
   **Absolute** layout mode, then click **Create**.
3. In the editor toolbar, click the **Terminal** icon (`{ }` on a
   document, immediately to the left of the `?` help icon). Replace all
   placeholder JSON with the contents of `aegis.json`, click **Apply and
   close**, then **Save** on the canvas.
4. Open the dashboard. Set time range to *Last 15 minutes* (or *Last 1
   hour*) and enable auto-refresh so panels populate while the daemon
   runs.

See the main [`README.md`](../README.md#b5-import-the-dashboard) for
full step-by-step screenshots guidance.

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
| Queue depth — 15-min forecast  | CDTSM prediction of gateway queue depth (requires B3.6 AI Toolkit + ~15 min data) |
| Dedup savings % — 15-min forecast | CDTSM prediction of dedup savings trend                                        |

**CDTSM panels empty or erroring?**

* **`Unknown search command 'apply'`** — install AI Toolkit ([B3.6](../README.md#b36-install-splunk-ai-toolkit-required--powers-cdtsm-forecast-panels)).
* **`Failed to retrieve tenant info: HTTP 404`** — **expected on local Splunk Enterprise.** CDTSM is Splunk Cloud / SLIM only. The other **9 panels** still work. See [`docs/splunk-blocker.md`](../docs/splunk-blocker.md).
* On **Splunk Cloud** with CDTSM enabled, run the [B3.6b smoke test](../README.md#b36b-smoke-test-cdtsm-splunk-cloud-only). Sourcetype must be `aegis:selfmetric` (colon, not underscore).

Do **not** use `| ai prompt=prompt` to validate CDTSM — that is a separate LLM command.

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
