# CDTSM forecast loop — closing the AI feedback cycle

> How Aegis uses the **Cisco Deep Time Series Model** (Splunk-Hosted)
> to predict its own future, then acts on that prediction.

## The pitch

Most observability projects use AI to *describe* what's happening now.
Aegis uses Splunk-Hosted CDTSM to **predict what will happen in the
next 15 minutes** and lets the AegisOps agent take pre-emptive action
before it does. The two metrics we forecast are:

| Metric | Forecasted because… | Action when prediction crosses threshold |
|---|---|---|
| `queue_depth` | When the SPSC ring fills, the gateway starts dropping anomalies (not just noise) | AegisOps proactively raises a noisy signature's dedup window before saturation hits |
| `dedup_savings_pct` | A sustained drop is the earliest leading indicator that an app deploy is generating new signatures the embedding model hasn't clustered yet | AegisOps replays raw events through the classifier to re-seed the suppression model |

This is a **closed-loop AI system** — the LLM (gpt-oss:20b via the
AITK `\| ai` SPL command) makes the *policy* decision after reading a
*forecast* produced by another Splunk-Hosted AI model (CDTSM). No
human is in the loop except as auditor.

## SPL the dashboard runs

Both forecast panels in `dashboards/aegis.json` use the canonical
CDTSM forecasting syntax from the [AI Toolkit 5.7.3 docs](https://help.splunk.com/en/splunk-cloud-platform/apply-machine-learning/use-ai-toolkit/5.7.3/ai-toolkit-models/feature-preview-cisco-deep-time-series-model):

```spl
index=aegis sourcetype=aegis:selfmetric
| timechart span=1m latest(queue_depth) AS queue_depth
| apply CDTSM queue_depth
    time_field=_time
    forecast_k=15
    conf_interval=90
    show_input=true
```

That returns a result row per timestamp with:

* `queue_depth` — observed values (from `show_input=true`)
* `predicted(queue_depth)` — CDTSM's point forecast for each of the
  next 15 minutes
* `lower90(predicted(queue_depth))` /
  `upper90(predicted(queue_depth))` — 90% confidence band

The Dashboard Studio line viz renders all of them on one plot.

## How AegisOps reads the forecast

The same SPL is configurable on the AegisOps agent side via the
config block:

```toml
[observe]
queue_forecast_spl = "index=aegis sourcetype=aegis:selfmetric | timechart span=1m latest(queue_depth) AS queue_depth | apply CDTSM queue_depth time_field=_time forecast_k=15 conf_interval=90"
queue_forecast_breach_threshold = 4096    # gateway's bus.capacity
```

Inside the observe-reason-act loop, after collecting the live gateway
status the agent issues this SPL via `SplunkClient.oneshot()`, finds
the maximum `predicted(queue_depth)` in the next 15 rows, and if it
exceeds `queue_forecast_breach_threshold`, includes a high-priority
hint in the reasoning prompt:

```text
The CDTSM forecast predicts queue_depth will hit {peak} in {minutes_to_peak}
minutes (capacity is {capacity}). Acting now to dampen the noisiest
signature should keep us below capacity.
```

The gpt-oss:20b model then almost always picks `override` on the
top-1 signature from the `aegis:metric` stream. The whole loop is
audited to `index=aegis sourcetype=aegis:agent` so the operator can
later replay why a particular suppression rule was raised before any
human saw a problem.

## Why CDTSM specifically (and not the legacy MLTK forecasters)

The hackathon **Best Use of Splunk Hosted Models** prize specifically
calls out the Cisco Deep Time Series Model (CDTSM). It's:

* A **foundation model** — no `| fit` step needed, the dashboard panel
  uses it inference-only with `| apply CDTSM`.
* **Splunk-Hosted** — runs inside the Splunk AI Toolkit runtime, no
  external service to provision.
* Designed for **multivariate** forecasting (we exploit this by
  potentially adding a third forecast on `events_in` to predict ingest
  spikes alongside the queue and savings forecasts).
* Confidence-aware — the `conf_interval` is the signal AegisOps uses to
  decide whether to act now (tight band, high confidence) or wait one
  more cycle (wide band, low confidence).

The 5.7+ syntax in this dashboard matches AITK's documented surface;
on earlier AITK versions you would use the DSDL/MLTK path
(`| fit MLTKContainer algo=ctsm_forecast …`) — see the *DSDL 5.2.3*
fallback notes in [`splunk-blocker.md`](splunk-blocker.md).
