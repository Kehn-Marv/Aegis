# Aegis Python AI sidecar

Optional FastAPI service the Rust gateway calls for higher-resolution
log analysis than structural hashing can provide. The gateway runs fine
without it — classification falls back to a keyword heuristic and the
sidecar's only job goes away — but turning it on unlocks the AI
classifier verdict on every collapsed event.

## Endpoints

| Method | Path             | Purpose                                                    |
|--------|------------------|-------------------------------------------------------------|
| `GET`  | `/health`        | Liveness probe                                             |
| `GET`  | `/info`          | Which embedding / hosted models are loaded                 |
| `POST` | `/embed`         | MiniLM (or fallback hash) embeddings for a batch of lines  |
| `POST` | `/cluster`       | KMeans cluster a batch of embeddings                       |
| `POST` | `/cluster_lines` | Embed + cluster in one call                                |
| `POST` | `/classify`      | Classify a log line as `anomaly` / `routine` / `unknown`   |

## Classification strategy

`/classify` tries strategies in order; each falls back to the next:

1. **Splunk Hosted Model (`| ai`)** — preferred when `AEGIS_SPLUNK_URL`
   and `AEGIS_SPLUNK_TOKEN` are set. Runs classification inside Splunk's
   search pipeline via the AI Toolkit `| ai` command. Hibernated when
   no Cloud / SLIM access — code path tested and reactivates on env-var
   change.
2. **OpenAI-compatible endpoint (e.g. local Ollama)** — when
   `AEGIS_HOSTED_MODEL_URL` is set. Point it at
   `http://127.0.0.1:11434/v1/chat/completions` and you get genuine
   LLM classification at the edge with zero Splunk dependencies.
3. **Embedding-distance** — cosine similarity between the line's
   sentence-transformer embedding and centroids built from canonical
   anomaly/routine seed phrases. Default day-to-day path: local,
   private, fast.
4. **Keyword heuristic** — final fallback so the API never returns
   `unknown` because nothing answered.

The response includes the `strategy` that actually produced the label
so dashboards can show which path each event took.

## Run locally

```powershell
cd sidecar
python -m venv .venv
.\.venv\Scripts\Activate.ps1
pip install -e . --extra-index-url https://download.pytorch.org/whl/cpu
python -m aegis_sidecar.server
# expect:  Uvicorn running on http://127.0.0.1:8765
```

> First `/classify` call lazy-downloads `sentence-transformers/all-MiniLM-L6-v2`
> (~80 MB). Subsequent calls are sub-millisecond. Offline machines fall
> back to a deterministic blake2b hash embedding so the API stays
> functional.

## Plan B: local Ollama (recommended)

Ollama exposes an OpenAI-compatible chat endpoint the existing adapter
already supports:

```powershell
ollama pull gpt-oss:20b           # ~13 GB on disk, ~16 GB RAM
# or lower-spec alternatives:
ollama pull qwen2.5:3b            # ~3 GB RAM
ollama pull gemma2:2b             # ~2 GB RAM

$env:AEGIS_HOSTED_MODEL_URL  = "http://127.0.0.1:11434/v1/chat/completions"
$env:AEGIS_HOSTED_MODEL_NAME = "gpt-oss:20b"
python -m aegis_sidecar.server
```

Verify:

```powershell
curl.exe http://127.0.0.1:8765/info
# hosted_model_transport: "openai_compat"
# hosted_model_name:      "gpt-oss:20b"
```

## Splunk Hosted Models (when available)

Same code path, different environment variables:

| Variable                  | Default         | Purpose                                      |
|---------------------------|-----------------|----------------------------------------------|
| `AEGIS_SPLUNK_URL`        | _unset_         | Splunk base URL                              |
| `AEGIS_SPLUNK_TOKEN`      | _unset_         | Auth token (needs `search` + `apply_ai_commander_command`) |
| `AEGIS_SPLUNK_AI_PROVIDER`| `splunk_hosted` | AITK provider name                           |
| `AEGIS_SPLUNK_AI_MODEL`   | `gpt-oss-20b`   | Model identifier                             |

Verify Splunk can run `| ai` before pointing the sidecar at it:

```spl
| makeresults
| eval prompt="Classify: ERROR connection refused"
| ai prompt=prompt provider=splunk_hosted model=gpt-oss-20b
```

## Environment-only knobs

| Variable                          | Default            | Purpose                          |
|-----------------------------------|--------------------|----------------------------------|
| `AEGIS_SIDECAR_PORT`              | `8765`             | Bind port                        |
| `AEGIS_SIDECAR_HOST`              | `127.0.0.1`        | Bind host                        |
| `AEGIS_EMBEDDING_MODEL`           | MiniLM-L6-v2       | Override the local embedder      |
| `AEGIS_HOSTED_MODEL_URL`          | _unset_            | OpenAI-compatible chat endpoint  |
| `AEGIS_HOSTED_MODEL_TOKEN`        | _unset_            | Bearer token (Ollama ignores)    |
| `AEGIS_HOSTED_MODEL_NAME`         | `gpt-oss-20b`      | Model identifier                 |
| `AEGIS_HOSTED_MODEL_TIMEOUT_SECS` | `6`                | Per-request timeout              |
