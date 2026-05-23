# Aegis AI — Splunk App

A Splunkbase-shaped Splunk app that exercises the **`splunklib.ai`**
agent SDK released with `splunk-sdk-python` to put two AI capabilities
directly inside Splunk, alongside Aegis edge-telemetry data.

This is the "AI for Splunk Apps - Build agentic workflows inside Splunk
apps using the Python SDK" capability called out in the
[Splunk Agentic Ops Hackathon resources](https://splunk.devpost.com/resources).

## What it ships

### 1. Custom Alert Action — `aegis_severity_assessment`

Triggered by a saved search that watches `sourcetype=aegis:metric` for
abnormally large suppressed-line counts. When it fires:

1. Reads the saved-search results from the gzip CSV temp file.
2. Builds an `AlertData` Pydantic object from the rows.
3. Calls `splunklib.ai.agent.Agent.invoke_with_data()` with a
   `SeverityAssessment` Pydantic `output_schema` so the LLM is
   *forced* to return a structured verdict.
4. Indexes the verdict to `sourcetype=aegis_ai:assessment` for the
   Aegis dashboard to pick up.

Result schema:

```jsonc
{
    "severity": "low|medium|high|critical",
    "confidence": 0.0-1.0,
    "summary": "2-3 sentences",
    "recommended_aegis_action": "noop|status|diagnostic|override",
    "recommended_duration_secs": 0-600,
    "rationale": "one sentence"
}
```

### 2. Custom Search Command — `| aegisreason`

A new SPL verb that enriches any pipeline with an LLM recommendation per
record. The intended use case is post-hoc reasoning over agent decisions:

```spl
index="aegis" sourcetype="aegis:agent" earliest=-1h
| head 10
| aegisreason context="fleet_operations"
| table _time gateway decision.action aegis_ai_recommendation_text
```

The command adds two fields per row:

* `aegis_ai_recommendation` — full JSON of the structured recommendation
* `aegis_ai_recommendation_text` — flat string suitable for `| table`

Recommendation schema:

```jsonc
{
    "next_action": "noop|status|diagnostic|override|reset",
    "duration_secs": 0-600,
    "confidence": 0.0-1.0,
    "rationale": "one sentence"
}
```

## LLM backend

Both entry points share `bin/llm_factory.py :: build_llm_model()`, which
constructs a `splunklib.ai.OpenAIModel`. Because `OpenAIModel` accepts
any OpenAI-compatible chat-completions endpoint, the app works
identically with:

| Backend | `AEGIS_AI_LLM_BASE_URL` | `AEGIS_AI_LLM_MODEL` |
|---|---|---|
| **Local Ollama** (default) | `http://127.0.0.1:11434/v1` | `gpt-oss:20b` |
| Ollama on host, Splunk in Docker | `http://host.docker.internal:11434/v1` | `gpt-oss:20b` |
| Splunk Hosted Models (when SLIM API is provisioned) | the SLIM SLIM-compatible OpenAI URL | `gpt-oss-20b` |
| Any vLLM / TGI / OpenAI deployment | that endpoint | per-deployment |

The default uses the same `gpt-oss:20b` model identifier Splunk Hosted
Models publishes — so when an environment with SLIM access becomes
available, *only the env-var changes* and the entire app starts using
Splunk-hosted gpt-oss-20b with **zero code modification**.

## Install on a real Splunk instance

```powershell
# 1. Copy this folder into the Splunk apps directory.
Copy-Item -Recurse apps\aegis_ai "$env:SPLUNK_HOME\etc\apps\aegis_ai"

# 2. (Once) install the Splunk Python SDK with its AI extras into the
#    splunklib search environment. On a fresh Splunk Enterprise 10.x
#    install the splunk-sdk-python package ships built-in; on older
#    builds you may need:
#       "$env:SPLUNK_HOME\bin\splunk" cmd python3 -m pip install --target "$env:SPLUNK_HOME\etc\apps\aegis_ai\bin\lib" "splunk-sdk[ai]>=2.0"

# 3. Set the LLM env vars (PowerShell example for an Enterprise instance).
[System.Environment]::SetEnvironmentVariable("AEGIS_AI_LLM_BASE_URL", "http://127.0.0.1:11434/v1", "Machine")
[System.Environment]::SetEnvironmentVariable("AEGIS_AI_LLM_MODEL",    "gpt-oss:20b",                "Machine")

# 4. Restart Splunk for the new app + env vars to be picked up.
& "$env:SPLUNK_HOME\bin\splunk" restart

# 5. Pull the model in Ollama (if you haven't already).
ollama pull gpt-oss:20b   # ~13 GB; for low-RAM dev: ollama pull qwen2.5:3b and override AEGIS_AI_LLM_MODEL

# 6. Enable the saved search.
#    Splunk Web -> Settings -> Searches, reports and alerts
#    -> "Aegis Severity Assessment" -> Edit Schedule -> turn schedule on.
```

The custom alert action will start firing within a minute of the next
collapsed-signature event passing the threshold.

## Verify the AI is wired correctly

```spl
| makeresults | eval search_name="manual_smoke", count=1234, signature="abc123"
| eval sample="ERROR connection refused to internal-api.local"
| eval classification.label="anomaly", classification.confidence=0.9, classification.strategy="embedding_distance"
| head 1
| aegisreason context="manual_smoke"
| table _time aegis_ai_recommendation_text
```

You should see a `noop` / `status` / `diagnostic` recommendation with a
short rationale in plain English within ~5 seconds of running the search
(longer on first invocation while Ollama loads the model).

## Packaging and AppInspect

The repository ships a vetted package and report:

| File | What it is |
|---|---|
| [`appinspect-report.json`](appinspect-report.json) | Latest local `splunk-appinspect inspect --mode test` output |
| `dist/aegis_ai.tar.gz` (built on demand) | The Splunkbase-installable tarball |

### Rebuild the tarball and re-validate

```powershell
# From the repo root:
py -c "import tarfile,os; root='apps/aegis_ai'; out='dist/aegis_ai.tar.gz'; os.makedirs('dist',exist_ok=True); skip={'tests','.pytest_cache','__pycache__','appinspect-report.json'}; tf=tarfile.open(out,'w:gz'); [tf.add(os.path.join(dp,f), arcname='aegis_ai/'+os.path.relpath(os.path.join(dp,f), root).replace(os.sep,'/')) for dp,_,fs in os.walk(root) if not any(s in dp.replace(os.sep,'/').split('/') for s in skip) for f in fs if f not in skip]; tf.close(); print('OK', os.path.getsize(out), 'bytes')"

splunk-appinspect inspect dist/aegis_ai.tar.gz --mode test --output-file apps/aegis_ai/appinspect-report.json
```

### Current AppInspect results

```
error:           0
failure:         0
future_failure:  0
warning:         5    (all environment-only; see below)
success:       104
```

The five remaining warnings are all **environment-only** (do not affect
Splunkbase acceptance):

1. `check_aarch64_compatibility` — skipped on Windows (will run on AppInspect Cloud).
2. `check_idx_binary_compatibility` — skipped on Windows.
3. `check_symlink_outside_app` — skipped on Windows.
4. `check_for_indexer_synced_configs` — `default/inputs.conf` only monitors
   the app's own log file (not synced to indexers in Splunk Cloud
   Victoria stack); this is intentional for a single-host app.
5. `check_for_python_script_existence` — Splunk Enterprise 8.0 cross-compat
   warning; AITK 5.6+ and Splunk Enterprise 9+/10+ are Python 3 only.

Run `splunk-appinspect inspect dist/aegis_ai.tar.gz --mode precert` or
upload to the [AppInspect API](https://dev.splunk.com/enterprise/reference/appinspect/appinspectapiepref/)
for the full Splunkbase pre-cert check.

## Troubleshooting

```spl
index="_internal" source="*aegis_ai.log"
```

surfaces every log line both entry points emit. The most common failure
modes are:

* `httpx.ConnectError: All connection attempts failed` — Ollama is not
  running, or `AEGIS_AI_LLM_BASE_URL` points at the wrong port.
* `KeyError: 'index'` in the alert action — the `output_index` in the
  saved-search `action.aegis_severity_assessment.param.output_index`
  doesn't exist; create it under *Settings → Indexes*.
* `pydantic.ValidationError` on the LLM reply — the model returned
  something off-schema. The agent SDK already retries internally; if
  you see this consistently, switch to a larger model
  (e.g. `gpt-oss:20b` → `qwen2.5:7b` is more reliable on small machines).
