# Aegis Python AI Sidecar

A FastAPI service the Rust gateway calls out to for higher-resolution log
analysis than structural hashing can provide.

## Endpoints

| Method | Path             | Purpose                                                    |
|--------|------------------|------------------------------------------------------------|
| `GET`  | `/health`        | Liveness probe                                             |
| `GET`  | `/info`          | Which embedding / hosted models are loaded                 |
| `POST` | `/embed`         | MiniLM (or fallback) embeddings for a batch of log lines   |
| `POST` | `/cluster`       | KMeans cluster a batch of embeddings, returns labels       |
| `POST` | `/cluster_lines` | Embed + cluster in one call (convenience for the gateway)  |
| `POST` | `/classify`      | Classify a log line as `anomaly` / `routine` / `unknown`   |

## Classification strategy

`/classify` tries three strategies in order; each falls back to the next:

1. **Splunk Hosted Model (`| ai`)** — preferred when `AEGIS_SPLUNK_URL`
   and `AEGIS_SPLUNK_TOKEN` are set. Runs classification inside Splunk's
   search pipeline via the AI Toolkit `| ai` command — the same
   transport the AegisOps agent uses for reasoning.
2. **OpenAI-compatible endpoint** — when `AEGIS_HOSTED_MODEL_URL` is set.
   Useful for local vLLM, TGI, or Ollama during offline development.
3. **Embedding-distance** — cosine similarity between the line's
   sentence-transformer embedding and centroids built from canonical
   anomaly/routine seed phrases. Default day-to-day path when Splunk is
   not configured: local, private, fast.
4. **Keyword heuristic** — last-resort signal so the API never returns
   `unknown` purely because nothing answered.

The response includes the `strategy` that actually produced the label
(`splunk_ai`, `openai_compat`, `embedding_distance`, or `keyword`) so
Splunk dashboards can show which path each event took.

## Run locally

```powershell
cd sidecar
uv venv
uv pip install -e .
uv run aegis-sidecar
```

Default address: `127.0.0.1:8765`. Override with `AEGIS_SIDECAR_HOST` /
`AEGIS_SIDECAR_PORT`.

## Splunk Hosted Model adapter (preferred)

Set these environment variables to route classification through Splunk
Hosted Models via SPL `| ai`:

| Variable                     | Default         | Purpose                                      |
|------------------------------|-----------------|----------------------------------------------|
| `AEGIS_SPLUNK_URL`           | _unset_         | Splunk base URL (no `/services/...` suffix)  |
| `AEGIS_SPLUNK_TOKEN`         | _unset_         | Auth token (`search` + `apply_ai_commander_command`) |
| `AEGIS_SPLUNK_AI_PROVIDER`   | `splunk_hosted` | AITK provider label from Connection Management |
| `AEGIS_SPLUNK_AI_MODEL`      | `gpt-oss-20b`   | Model identifier                             |
| `AEGIS_SPLUNK_VERIFY_TLS`    | `true`          | TLS verification                             |
| `AEGIS_SPLUNK_TIMEOUT_SECS`  | `12`            | Per-request timeout                          |

Verify Splunk can run `| ai` before pointing the sidecar at it:

```spl
| makeresults
| eval prompt="Classify: ERROR connection refused"
| ai prompt=prompt provider=splunk_hosted model=gpt-oss-20b
```

## OpenAI-compatible fallback

For offline development without Splunk credentials:

| Variable                          | Default            | Purpose                          |
|-----------------------------------|--------------------|----------------------------------|
| `AEGIS_HOSTED_MODEL_URL`          | _unset_            | Chat-completions endpoint        |
| `AEGIS_HOSTED_MODEL_TOKEN`        | _unset_            | Bearer token                     |
| `AEGIS_HOSTED_MODEL_NAME`         | `gpt-oss-20b`      | Model identifier                 |
| `AEGIS_HOSTED_MODEL_TIMEOUT_SECS` | `6`              | Per-request timeout              |
| `AEGIS_EMBEDDING_MODEL`           | MiniLM-L6-v2       | Override the local embedder      |

## Tests

```powershell
uv pip install -e ".[dev]"
uv run pytest
```

The classifier tests exercise the keyword and embedding-distance paths
through a deterministic hash-based fallback embedder, so they pass
offline without downloading sentence-transformers.
