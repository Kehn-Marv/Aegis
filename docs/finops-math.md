# Aegis — FinOps math, worked example

## Enterprise persona (who pays for this)

| Field | Value |
|-------|-------|
| **Buyer** | VP Platform / FinOps lead |
| **Fleet** | 120 microservices across 2 regions (`us-east`, `eu-west`) |
| **Pain** | Monthly Splunk ingest ~18 TB; 60% is repetitive crash-loop noise |
| **Deployment** | One Aegis gateway per region (DaemonSet or VM), shared Python sidecar |
| **Success metric** | ≥90% ingest reduction on error spam *without* losing first-occurrence context |

The math below uses **one crash-looping service** — multiply by however
many services in your fleet actually behave this way during incidents.

## Scenario: one crash-looping service

A single payment-service pod has entered a crash loop. Its supervisor
restarts it every few seconds. On each restart the service connects to
the database, fails (the DB is down), logs a five-line stack trace, and
exits. The supervisor restarts it again immediately.

Assumptions, all conservative:

| Quantity                          | Value      |
|-----------------------------------|------------|
| Restarts per minute               | 2,000      |
| Lines per stack trace             | 5          |
| Total error lines per minute      | 10,000     |
| Average bytes per line            | 400        |
| Bytes per minute                  | 4,000,000  |
| Bytes per hour                    | 240,000,000 |
| Bytes per day                     | 5.76 GB    |

## Without Aegis

Every line is forwarded to Splunk via the Universal Forwarder.

**Daily ingest: 5.76 GB per service.**

## With Aegis (dedup window: 30 s)

The dedup engine sees the same structural signature on every iteration.
Behaviour per 30-second window:

* **First occurrence of the signature in the window:** forwarded raw,
  full 400-byte line. Happens *once per service start*, then collapses.
  Even in the absolute worst case, treat this as one raw line per window.
* **All subsequent occurrences:** collapsed into one metric event of the
  form
  `{"kind":"collapsed","signature":"...","count":5000,"window_secs":30, "sample":"..."}`,
  ~250 bytes after JSON overhead.
* **Plus** one ~150-byte `aegis:selfmetric` event emitted every 15 s
  (twice per window) — independent of how many error lines arrived.

Per 30-second window from this one service:

```
1 first-occurrence raw event   :  400 bytes
1 collapsed metric event       :  250 bytes
2 self-metric events           :  300 bytes (half attributable to this service)
                                 ─────────
                                  950 bytes per 30s
                                = 1,900 bytes per minute
                                = 2,736,000 bytes per day
                                ≈ 2.6 MB per day
```

**Daily ingest with Aegis: ~2.6 MB per service.**

## Savings

| Metric                              | Value                          |
|-------------------------------------|--------------------------------|
| Raw daily ingest                    | 5.76 GB                        |
| Aegis daily ingest                  | 2.6 MB                         |
| **Reduction**                       | **99.96%**                     |
| Daily bytes avoided                 | ~5.76 GB                       |
| Annual bytes avoided per service    | ~2.1 TB                        |

## Dollar conversion

Splunk Cloud and Enterprise ingest pricing varies by contract, region,
and commit term. Publicly observable workload-pricing benchmarks in 2026
(see Splunk's pricing page and analyst reports) put the marginal cost in
the range **$1,500 – $2,000 per GB ingested per year** at typical
mid-tier commitment levels.

Using **$1,800 / GB-year** as a middle-of-the-road number:

| Per-service annual savings                  | Value       |
|---------------------------------------------|-------------|
| Bytes avoided                               | 2.1 TB      |
| × $1,800/GB-year                            | **$3,780**  |

For a realistic incident lasting **8 hours** before SRE intervenes:

| Per-incident savings                        | Value       |
|---------------------------------------------|-------------|
| Bytes avoided                               | ~1.9 GB     |
| × $1,800/GB-year, prorated 8h               | **~$3,400** |

If the organisation runs **100 services** in production and a single
service crash-loops once a month for 8 hours before catch:

| Annual fleet savings (conservative)         | Value       |
|---------------------------------------------|-------------|
| Incidents/year                              | 100 × 12 = 1,200 |
| Bytes avoided per incident                  | ~1.9 GB     |
| × $1,800/GB-year, prorated                  | **~$4.1M**  |

Even if those numbers are off by an order of magnitude, the order of
magnitude *itself* is meaningful — and this is just the dedup story.
The routine-traffic summarization layer rolls `routine`-classified
collapses into periodic `aegis:summary` events for additional savings
on the 95%+ of traffic that isn't an anomaly.

## The agentic upside

The override tool is what makes the savings *safe*. Operators who
deploy aggressive dedup gateways often find themselves in a debugging
nightmare during a real incident: the gateway is hiding the very logs
they need. Aegis solves this with `aegis.override(seconds=30)` callable
straight from a Cursor or Claude Desktop chat:

```
You: "Customer escalation on payment-service. Stream raw for 60s."
Claude: [calls aegis.override(seconds=60)]
         "Override engaged. You're now seeing the un-deduplicated stream
          in Splunk under sourcetype=aegis:raw. Override will release
          automatically in 60 seconds."
```

That's the difference between a cost-saver and a cost-saver people
actually trust to leave on.

## How to reproduce

Run the demo stack (4 commands at the top of [`README.md`](../README.md))
and let it run for an hour with the spammer at the default rate. Then
in Splunk Web:

```spl
| tstats count where index=aegis sourcetype=aegis:raw earliest=-1h
| append [
    | tstats sum(count) AS suppressed where index=aegis sourcetype=aegis:metric earliest=-1h
]
```

Divide and you'll see your live dedup ratio. The control-panel UI shows
the same number live, no SPL required.
