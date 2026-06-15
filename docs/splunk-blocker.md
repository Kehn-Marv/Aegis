# Splunk Hosted Models  -  provisioning blocker and the three LLM transports

> Honest writeup of an infrastructure wall hit during the Splunk
> Agentic Ops Hackathon 2026 and the **two** architectural pivots that
> keep Aegis fully functional end-to-end.

## TL;DR

We originally targeted Splunk-Hosted Models (`gpt-oss-20b` via the
AI Toolkit `| ai` SPL command, SLIM-backed). The 14-day Splunk Cloud
trial does **not** provision SLIM, so direct Hosted-Models calls
return HTTP 500.

We solved this **twice** in code, giving us three LLM transports (one
default, two opt-in):

| Transport | Where the model runs | Splunk surface? | Trial-safe? | Default? |
|---|---|---|---|---|
| **`ollama`** | Local Ollama next to the gateway | Bypassed | ✅ Yes | ✅ Yes (edge-first) |
| **`aitk_ollama`** | Local Ollama, called *through* Splunk AITK `\| ai` | ✅ SPL `\| ai` command, audited in `_audit` | ✅ Yes (Developer License) | No (opt-in) |
| **`splunk_ai`** | Splunk-Hosted Models (SLIM) `gpt-oss-20b` / `Foundation-Sec-1.1-8B` | ✅ SPL `\| ai` command | ❌ Gated on trial | No (provisioned envs only) |

The whole transport surface is one config flag (`[llm].transport =
"ollama" | "aitk_ollama" | "splunk_ai"`). All three share the same
`SplunkAITransport` and `OllamaTransport` implementations so a
provisioned Splunk environment can adopt any path without code changes.

## What we actually ran vs what we built

Honest split for reviewers — the project is end-to-end real; only the
**Splunk AI routing layer** differed from the original plan.

| Piece | Built in repo? | Ran in our hackathon env? | Notes |
|---|---|---|---|
| Gateway → HEC (8 sourcetypes) | ✅ | ✅ | Primary Splunk integration |
| AegisOps agent + Ollama (`transport = "ollama"`) | ✅ | ✅ | **Default demo path** |
| Aegis AI app (`splunklib.ai` → Ollama HTTP) | ✅ | ✅ | Direct Ollama, not `\| ai` |
| Splunk MCP client (agent observability) | ✅ | ✅ | When Splunk creds configured |
| AITK `\| ai` via Ollama connection (`aitk_ollama`) | ✅ | ❌ | Code + docs; needs AITK **Ollama** connection (different from the SLIM connection we tried) |
| Splunk Hosted Models / SLIM (`splunk_ai`) | ✅ | ❌ | Blocked on 14-day Cloud trial |
| CDTSM forecast panels | ✅ | ❌ | Cloud-only; panels show expected 404 locally |

We did **not** mock the LLM or the Splunk telemetry pipeline. We ran a
real local model and shipped real audit events. What we could not turn on
was Splunk's `\| ai` command — first because SLIM provisioning failed,
then because we fell back to direct Ollama rather than completing a
separate AITK **Ollama-type** connection setup.

## What we attempted (direct SLIM path)

End-to-end, in order:

1. **Local Splunk Enterprise + Developer License**  -  confirmed
   SLIM-backed Hosted Models are Cloud-only.
2. **Splunk Cloud 14-day trial**  -  provisioned, logged in as
   `sc_admin`.
3. **AI Toolkit app**  -  installed from Splunkbase. Upgraded to 5.7.4.
4. **Splunk AI Assistant app**  -  installed to trigger the global AI
   Terms of Service prompt. Accepted the ToS.
5. **`apply_ai_commander_command` capability**  -  verified granted to
   `sc_admin`.
6. **Connections → New Connection → LLM → Splunk Hosted (SLIM API)**  - 
   provider dropdown said *No providers found*. Forcing a custom LLM
   connection threw `HTTP 500 (Internal Server Error)` on
   `/services/configs/sc_admin` and `404 (Not Found)` on
   `/servicesNS/...` from the AI Toolkit's own React app
   (DevTools console verified).
7. **Search & Reporting `| makeresults | ai prompt=...`**  -  same
   underlying error: no SLIM provider wired up.

The 14-day automated trial's REST API is locked down to prevent
abuse of the SLIM API. Provisioning Hosted Models on these trials
requires a manual flip by a Splunk sales engineer or hackathon
organiser.

## Plan A′  -  AITK Connection Management + local Ollama (implemented, opt-in)

This is the path we should have started with — and the one we **did not
complete on our trial box** after the SLIM connection failed. AITK 5.6+ supports
**user-defined LLM connections**, and Ollama is one of the supported
provider types out of the box (per the Lantern doc *Leveraging
generative AI capability in security operations with the AITK*,
section "Setup for the AITK AI command", which explicitly calls out
Ollama as a supported on-prem LLM).

So we get the **full `| ai` SPL experience**  -  audited in `_audit`,
reproducible from saved searches, embeddable in dashboards  -  with the
LLM call ultimately served by local Ollama on the same machine as
Splunk Enterprise. No SLIM API. No 14-day trial gate.

Full setup walkthrough: **[`docs/aitk-ollama.md`](aitk-ollama.md)**.

To use this transport, set in `agent/configs/aegis-ops.toml`:

```toml
[llm]
transport = "aitk_ollama"

[llm.aitk_ollama]
provider = "ollama_local"    # AITK connection name
model    = "gpt-oss:20b"

[splunk]
url   = "https://localhost:8089"
token = "your-splunk-auth-token-with-search"
```

The agent starts logging:

```
INFO SplunkAITransport initialised: provider=ollama_local model=gpt-oss:20b
```

…and every reasoning call now flows through Splunk's `| ai` command.
This is a genuine integration with the AI Toolkit and the SPL `| ai`
surface.

## Plan B  -  raw Ollama (default — what we ran)

The original Plan B from the first iteration of this doc. This is what
we **actually used** for the hackathon demo after AITK Connection
Management could not provision SLIM. It is also the **edge-first
default**: when the agent is deployed *at* an edge site (where the whole
point is to keep traffic off the WAN), routing LLM calls back to a
centralised Splunk Cloud is the opposite of what we want. So:

* **`transport = "ollama"`** stays the default in
  `agent/configs/aegis-ops.example.toml`.
* It's the only transport that needs zero Splunk credentials at all.
* It's also the only transport that survives an offline edge site.

The Aegis AI Splunk app (`apps/aegis_ai/`) similarly uses raw Ollama
via `splunklib.ai.OpenAIModel(base_url=…)` because the
`splunklib.ai.Agent` SDK doesn't itself route through AITK  -  it speaks
OpenAI-compatible HTTP directly. (See
[`docs/aitk-ollama.md`](aitk-ollama.md), section "Wire it into the
Aegis AI Splunk app", for the rationale.)

### Model selection by RAM

| System RAM | Recommended model | Active RAM | Notes |
|------------|-------------------|------------|-------|
| **16 GB+** | `gpt-oss:20b` (default) | ~13 GB | Matches the Splunk Hosted Models name |
| 8–16 GB | `qwen2.5:7b` | ~5 GB | Strong reasoning at moderate RAM |
| **6–8 GB** | `qwen2.5:3b` | ~3 GB | Qwen 2.5 is explicitly tuned for JSON |
| 4–6 GB | `gemma2:2b` | ~2 GB | Smaller but solid |
| <4 GB | `qwen2.5:1.5b` | ~1.5 GB | Basic but functional |

## What's preserved for the SLIM path (`transport = "splunk_ai"`)

* `agent/aegis_ops/transports.py :: SplunkAITransport`  -  full
  `| ai` SPL builder + `oneshot` plumbing.
* `agent/aegis_ops/splunk_client.py`  -  `oneshot()` + `HecClient` used
  by `SplunkAITransport` and the audit pipeline.
* `sidecar/aegis_sidecar/splunk_ai.py`  -  sidecar classifier transport
  for `| ai`. Wired through `hosted_model.py`.
* All env-var contracts (`AEGIS_SPLUNK_URL`, `AEGIS_SPLUNK_TOKEN`,
  `AEGIS_SPLUNK_AI_*`) preserved.

Switching to true Splunk-Hosted Models on a provisioned account:

```toml
[llm]
transport = "splunk_ai"

[llm.splunk_ai]
provider = "splunk_hosted"      # AITK provider name on a SLIM account
model    = "gpt-oss-20b"        # or "gpt-oss-120b" or "Foundation-Sec-1.1-8B-Instruct"
```

No code changes, no redeployment, no UI changes. Same prompt, same
decision schema, same audit trail  -  only `provider` differs from the
`aitk_ollama` shape.

## Effect on Splunk AI Assistant 2.0 integration

SAIA 2.0 uses the same SLIM infrastructure as the `| ai` command
*when configured against `splunk_hosted`*. On a trial account where
SLIM is gated, SAIA cannot answer generative-AI prompts either.

What still works in the trial:

* SAIA's SPL-search surface (it can still issue SPL queries on the
  user's behalf even without generative reasoning).
* Any SPL search against `index=aegis sourcetype=aegis:agent`  - 
  including the recommended operator workflow in
  [`docs/saia-integration.md`](saia-integration.md).
* The new Aegis-AI custom alert action and `| aegisreason` custom
  search command from `apps/aegis_ai/`  -  these don't depend on SAIA
  at all; they use `splunklib.ai.Agent` directly.

What doesn't:

* SAIA replying in natural language about Aegis audit events.

## What this means for the submission

Aegis is a **real** end-to-end system — gateway, agent, Splunk app,
HEC audit trail, and a **live local LLM** — not a mocked demo.

What we can claim honestly:

* **Ran live:** Ollama-backed reasoning (agent + Splunk app), full HEC
  pipeline, MCP observability, Dashboard Studio over indexed Aegis data.
* **Implemented, one config line away:** the `| ai` SPL transport
  (`SplunkAITransport`, `sidecar/splunk_ai.py`) for both
  `aitk_ollama` (AITK Ollama connection) and `splunk_ai` (SLIM Hosted
  Models). Same prompt, same `Decision` schema, same audit shape.
* **Blocked on our trial environment:** SLIM provisioning (HTTP 500 /
  "No providers found" in AITK Connection Management) and therefore
  `\| ai` searches with no default provider. We did **not** subsequently
  stand up an AITK **Ollama-type** connection on that box.
* **Tested without live SLIM:** `SplunkAITransport` plumbing against a
  mocked `\| ai` oneshot response during development.

That is materially different from "we never integrated Hosted Models"
*or* "we faked the AI layer". We integrated the Splunk AI surfaces in
code, ran genuine LLM reasoning via direct Ollama, and kept the same
`gpt-oss:20b` model identifier so a provisioned SLIM or AITK Ollama
connection is a config flip, not a rewrite.
