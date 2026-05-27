# AITK + Ollama — bringing `| ai` SPL to a Developer-License Splunk

> The third live LLM transport for Aegis.

## Why this exists

The Splunk AI Toolkit (AITK) v5.6+ introduced **LLM Connectors** in its
Connection Management UI. These connectors let SPL `| ai` route to any
of the LLM providers AITK supports — including **local Ollama**.

That gives us a path to demonstrate "Best Use of Splunk Hosted Models"
*without* needing SLIM API access on a 14-day Splunk Cloud trial:

```
SPL search  →  | ai prompt=... provider=ollama_local model=gpt-oss:20b
                  │
                  ▼
            AITK runtime (inside Splunk)
                  │
                  ▼
            Ollama (localhost on the same machine as Splunk)
                  │
                  ▼
            gpt-oss:20b reply  →  back as a result row
```

This is materially different from our raw-Ollama transport because the
LLM call happens **inside Splunk's search pipeline** — every invocation
is automatically logged in `_audit`, the SPL itself is reproducible from
the dashboard, and switching to true Splunk Hosted Models later is a
one-word change (`provider=ollama_local` → `provider=splunk_hosted`).

## One-time setup on Splunk Enterprise

### 1. Install dependencies

| What | How |
|---|---|
| Splunk Enterprise 10.x (Developer License) | Path B in [`../README.md`](../README.md#path-b--live-mode-with-splunk-enterprise-30-minutes) |
| Python for Scientific Computing v4.2.3 | Splunkbase → install, restart Splunk |
| Splunk AI Toolkit 5.6+ | Splunkbase → install, restart Splunk |
| Ollama on the same host | <https://ollama.com/download> |
| `gpt-oss:20b` pulled in Ollama | `ollama pull gpt-oss:20b` (~13 GB; for low-RAM: `ollama pull qwen2.5:3b`) |

### 2. Verify your user has `apply_ai_commander_command`

```
Splunk Web → Settings → Access Controls → Roles → <your role> → Capabilities
```

Search for `apply_ai_commander_command`. If absent, add it. (Admin and
power user typically have it; bare user roles don't.)

### 3. Create the Ollama LLM connection in AITK

```
Splunk Web → AI Toolkit (`Splunk AI Toolkit`) → Connection Management
            → New LLM Connection
                    Type:           Ollama
                    Name:           ollama_local
                    Endpoint URL:   http://127.0.0.1:11434
                    Default model:  gpt-oss:20b
            → Save
```

The connection name (`ollama_local`) is what you'll pass as `provider=`
in the `| ai` SPL command. Keep it short and lowercase to avoid quoting.

### 4. Smoke-test the wiring from Splunk Web

```spl
| makeresults
| eval prompt="Reply with the single word 'pong'."
| ai prompt=prompt provider=ollama_local model=gpt-oss:20b
```

You should get back a row with an `ai_result_1` field containing `pong`
within ~3 seconds of the first call (longer on the very first one while
Ollama loads the model into memory). If you get a `provider not found`
error, the connection name doesn't match the `provider=` value in the
SPL — re-check step 3.

## Wire it into AegisOps Agent

In `agent/configs/aegis-ops.toml`:

```toml
[llm]
transport = "aitk_ollama"

[llm.aitk_ollama]
provider     = "ollama_local"   # must match the AITK connection name
model        = "gpt-oss:20b"    # must match the AITK default model
timeout_secs = 30

[splunk]
url        = "https://localhost:8089"
token      = "PASTE-A-SPLUNK-AUTH-TOKEN-WITH-SEARCH-CAPABILITY"
verify_tls = false
```

Then `aegis-ops --config configs/aegis-ops.toml --once -v` and look
for this startup line. On CPU Ollama with `transport = "ollama"`, allow
~5 minutes per gateway — see [`../Troubleshooting.md`](../Troubleshooting.md).

```
INFO AegisOps starting: ... llm=splunk_ai splunk=on audit=...
INFO SplunkAITransport initialised: provider=ollama_local model=gpt-oss:20b
```

Every reasoning step now flows through:

```
AegisOps → /services/search/jobs/oneshot
         → | ai prompt=... provider=ollama_local model=gpt-oss:20b
         → AITK → Ollama → reply → result row → AegisOps
```

## Wire it into the Aegis AI Splunk app (`apps/aegis_ai/`)

The `splunklib.ai.OpenAIModel` used by the alert action and the
`| aegisreason` custom search command bypasses AITK and talks to Ollama
directly via its OpenAI-compatible endpoint. This is by design: the
`splunklib.ai.Agent` SDK already abstracts the LLM provider, so
double-routing through AITK would just add latency.

If you'd rather route the app through AITK too (e.g. to centralise
auditing in `_audit`), point `AEGIS_AI_LLM_BASE_URL` at AITK's own
chat-completions surface when that becomes available. As of AITK
5.7.4 / May 2026, AITK does not expose an OpenAI-compatible HTTP
endpoint of its own — only the SPL `| ai` command — so the app uses
Ollama's OpenAI surface directly.

## Migrating to true Splunk Hosted Models later

When you (or a customer) have a Splunk Cloud account with SLIM API
access, the change is a single line in the agent config:

```toml
[llm]
transport = "splunk_ai"

[llm.splunk_ai]
provider = "splunk_hosted"
model    = "gpt-oss-20b"
```

The SPL the transport builds is identical except for the provider
name. No code changes anywhere in the agent. The Aegis AI app
similarly migrates by changing `AEGIS_AI_LLM_BASE_URL` to point at the
SLIM endpoint (and the model id from `gpt-oss:20b` to `gpt-oss-20b`).
