# Aegis Splunk dashboard

`aegis.json` is a Splunk Dashboard Studio definition that visualises the
four pillars of Aegis: noise gate, causal chains, incident memory, and
decision cards.

## Install

1. In Splunk Web, open **Search & Reporting → Dashboards → Create New
   Dashboard**.
2. Title `Aegis`, leave Permissions Private, pick **Dashboard Studio**,
   choose **Absolute** layout, click **Create**.
3. In the editor click the **Terminal** icon (`{ }`), paste the entire
   contents of `aegis.json`, click **Apply and close**, then **Save**.
4. Set the time range to *Last 15 minutes* and turn on auto-refresh.

## Panel groups

| Group              | What it tells you                                                                  |
|--------------------|------------------------------------------------------------------------------------|
| Top KPIs           | Health state, noise stopped %, incidents remembered, queue depth                  |
| Decision cards     | Every focused recommendation Aegis surfaced (state, root cause, headline)         |
| Causal chains      | Which service broke first in each multi-service incident                          |
| Incident memory    | Stored fingerprints  -  resolved ones carry the cause + fix the engineer recorded   |
| Noise gate         | Ingest vs forwarded chart, top suppressed signatures, classifier verdict          |
| First-occurrence   | Rate of new signatures  -  spikes mean deploys or incidents                         |
| Silent services    | Services that were talking and went quiet                                         |
| CDTSM forecasts    | Splunk-Hosted CDTSM 15-min forecasts of queue depth + dedup savings (Cloud only)  |

## Sourcetypes consumed

| sourcetype          | when                                                  |
|---------------------|-------------------------------------------------------|
| `aegis:raw`         | first occurrence of a signature; override passthrough |
| `aegis:metric`      | dedup window closed for a repeating signature         |
| `aegis:summary`     | routine-classified traffic, rolled up                 |
| `aegis:causal`      | multi-service incident with a probable root cause     |
| `aegis:decision`    | the focused "next step" card the engineer reads       |
| `aegis:incident`    | a memory entry (fingerprint + resolution when set)    |
| `aegis:silent`      | a service that was talking has gone quiet             |
| `aegis:selfmetric`  | gateway self-metrics emitted every ~15s               |
| `aegis:agent`       | AegisOps Agent decision audit                         |
| `aegis_ai:assessment` | LLM verdict from the Splunkbase app's alert action  |

## Quick SPL crib

```spl
# Money-shot: lines suppressed in the last hour
index=aegis sourcetype=aegis:metric
| stats sum(count) AS lines_suppressed

# Most recent causal chains and who broke first
index=aegis sourcetype=aegis:causal
| sort - _time
| table _time root_cause_service services_involved confidence

# Past incidents with a recorded fix (the institutional memory)
index=aegis sourcetype=aegis:incident resolved=true
| sort - resolved_at
| table resolved_at root_cause_service cause fix resolved_in_minutes
```

## CDTSM panels empty?

* `Unknown search command 'apply'` → install Splunk AI Toolkit.
* `Failed to retrieve tenant info: HTTP 404` → CDTSM is Splunk Cloud /
  SLIM only. The other panels still populate. See
  [`../docs/splunk-blocker.md`](../docs/splunk-blocker.md).
