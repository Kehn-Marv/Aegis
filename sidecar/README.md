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

`/classify` tries strategies in order; each falls back to the next:

1. **Splunk Hosted Model (`| ai`)** — preferred when `AEGIS_SPLUNK_URL`
   and `AEGIS_SPLUNK_TOKEN` are set. Runs classification inside Splunk's
   search pipeline via the AI Toolkit `| ai` command. **Currently
   hibernated** because the hackathon's Splunk Cloud trial does not
   provision the SLIM API — see
   [`../docs/splunk-blocker.md`](../docs/splunk-blocker.md). Code path
   is preserved and tested; setting the env vars re-activates it when
   SLIM access lands.
2. **OpenAI-compatible endpoint (incl. local Ollama)** — when
   `AEGIS_HOSTED_MODEL_URL` is set. **This is the recommended path
   today**: point it at a local Ollama server running `gpt-oss:20b` and
   you get genuine LLM classification at the edge with zero Splunk
   dependencies.
3. **Embedding-distance** — cosine similarity between the line's
   sentence-transformer embedding and centroids built from canonical
   anomaly/routine seed phrases. Default day-to-day path when nothing
   else is configured: local, private, fast.
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

## Plan B: local Ollama (recommended today)

Ollama exposes an OpenAI-compatible chat-completions endpoint that the
existing adapter already supports. Install Ollama, pull the model, and
point the sidecar at it:

```powershell
# 1. Install Ollama from https://ollama.com/download then:
ollama pull gpt-oss:20b     # ~13 GB on disk, ~16 GB RAM (matches Splunk Hosted Models name)
# Lower-spec alternatives: ollama pull qwen2.5:3b (~3 GB RAM) or ollama pull gemma2:2b (~2 GB RAM)

# 2. Point the sidecar at it (PowerShell):
$env:AEGIS_HOSTED_MODEL_URL  = "http://127.0.0.1:11434/v1/chat/completions"
$env:AEGIS_HOSTED_MODEL_NAME = "gpt-oss:20b"
uv run aegis-sidecar
```

Verify:

```powershell
curl.exe http://127.0.0.1:8765/info
# hosted_model_transport: "openai_compat"
# hosted_model_name:      "gpt-oss:20b"
```

The Aegis gateway will now annotate every collapsed event with a real
LLM classification, generated entirely on your machine.

## Splunk Hosted Model adapter (hibernated)

When a Splunk Cloud account with SLIM API access becomes available,
set these environment variables to route classification through
Splunk Hosted Models via SPL `| ai`:

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

<!-- Tests are intentionally not committed to this repo (see .gitignore).
     Devs adding tests locally can install dev deps with:
       uv pip install -e ".[dev]"
     and run them with `uv run pytest`. -->

