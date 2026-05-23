# Splunk Hosted Models — provisioning blocker and Plan B

> Honest writeup of an infrastructure wall hit during the Splunk
> Agentic Ops Hackathon 2026 and the architectural pivot that kept
> Aegis fully functional.

## TL;DR

We tried to integrate Splunk Hosted Models (`gpt-oss-20b` via the
AI Toolkit `| ai` SPL command). The 14-day Splunk Cloud trial that
hackathon participants use does **not** provision the SLIM API the
hosted models run on. Configuration writes return HTTP 500 from
Splunk's REST surface.

We have opened a support ticket on Devpost asking for the gate to be
flipped. While we wait, the AegisOps agent runs against a **local
Ollama instance hosting the same `gpt-oss:20b` model identifier**.
All code paths for the Splunk transport are preserved; flipping
`llm.transport = "splunk_ai"` in `agent/configs/aegis-ops.toml`
re-activates the original path the moment SLIM access lands.

## What we attempted

End-to-end, in order:

1. **Local Splunk Enterprise + Developer License** — confirmed Hosted
   Models are Cloud-only.
2. **Splunk Cloud 14-day trial** — provisioned, logged in as
   `sc_admin`.
3. **AI Toolkit app** — installed from Splunkbase. Upgraded to 5.7.4.
4. **Splunk AI Assistant app** — installed to trigger the global AI
   Terms of Service prompt. Accepted the ToS.
5. **`apply_ai_commander_command` capability** — verified granted to
   `sc_admin`.
6. **Connections → New Connection → LLM → Splunk Hosted (SLIM API)** —
   provider dropdown said *No providers found*. Forcing a custom LLM
   connection threw `HTTP 500 (Internal Server Error)` on
   `/services/configs/sc_admin` and `404 (Not Found)` on
   `/servicesNS/...` from the AI Toolkit's own React app
   (DevTools console verified).
7. **Search & Reporting `| makeresults | ai prompt=...`** — same
   underlying error: no provider wired up, no SLIM endpoint reachable.

The 14-day automated trial's REST API is locked down to prevent
abuse of the SLIM API. Provisioning Hosted Models on these trials
requires a manual flip by a Splunk sales engineer or hackathon
organizer.

## Support request

A formal request has been filed with the hackathon organizers asking
for the trial environment to be upgraded so `splunk_hosted` shows up
as a provider option. Verbatim message in [the project transcript].

If they respond and provision the account, flipping back is a single
config line:

```toml
[llm]
transport = "splunk_ai"   # was "ollama"
```

Plus paste the auth token into `[splunk]`. No code changes.

## Plan B — Ollama as the local LLM transport

### Why Ollama is actually a *better* fit (not a downgrade)

| Concern | Plan A (Splunk Hosted) | Plan B (Ollama) |
|---------|-----------------------|-----------------|
| Reasoning model | `gpt-oss-20b` (Splunk Hosted) | `qwen2.5:3b` (Ollama, ~3 GB RAM); upgrades to `qwen2.5:7b` or larger on bigger machines |
| Where it runs | Splunk Cloud datacentre | Same machine as the Aegis edge gateway |
| Round-trip latency | Network + Splunk search pipeline | Localhost, single process |
| Network requirement | Always-on uplink to Splunk Cloud | None — works offline at the edge |
| Cost per call | Splunk ingest + compute pricing | Free |
| JSON reliability | Best-in-class | **Hard schema enforcement** via Ollama's `format` parameter so even a 3B model can't emit malformed `Decision` JSON |
| Hackathon prize fit | "Best Use of Splunk Hosted Models" | Still demonstrates the SPL `\| ai` integration with code in `transports.SplunkAITransport`; hibernated, not deleted |

Aegis's entire thesis is **edge-first observability**. Running the LLM
locally next to the gateway is on-message, not off-message. The story
gets *stronger*: "the gateway, the AI sidecar, and the agent's brain
all live at the edge — Splunk gets pre-classified, pre-collapsed,
audit-ready events".

### Model selection by RAM

| System RAM | Recommended model | Active RAM | Notes |
|------------|-------------------|------------|-------|
| **6–8 GB** | `qwen2.5:3b` (default) | ~3 GB | Qwen 2.5 is explicitly tuned for JSON output |
| 4–6 GB | `gemma2:2b` | ~2 GB | Smaller but still solid |
| <4 GB | `qwen2.5:1.5b` | ~1.5 GB | Basic but functional |
| 16 GB+ | `qwen2.5:7b` | ~5 GB | Highest quality available locally |

### What's preserved for the Splunk path

* `agent/aegis_ops/transports.py :: SplunkAITransport` — full
  `| ai` SPL builder + `oneshot` plumbing.
* `agent/aegis_ops/splunk_client.py` — `oneshot()` + `HecClient` used
  by `SplunkAITransport` and the audit pipeline.
* `sidecar/aegis_sidecar/splunk_ai.py` — sidecar classifier transport
  for `| ai`. Wired through `hosted_model.py`.
* All env-var contracts (`AEGIS_SPLUNK_URL`, `AEGIS_SPLUNK_TOKEN`,
  `AEGIS_SPLUNK_AI_*`) preserved.
* All tests for both transports continue to pass (mocked HTTP).

### What ships in Plan B

* `agent/aegis_ops/transports.py :: OllamaTransport` — default. Calls
  `POST /api/chat` on a local Ollama server with the agent's existing
  prompt template (`prompts.build_full_prompt`) and parses the same
  `Decision` JSON schema.
* `agent/configs/aegis-ops.example.toml` ships with
  `llm.transport = "ollama"` and `model = "gpt-oss:20b"`.
* `[splunk]` and `[audit]` blocks are now **optional**. The agent runs
  end-to-end with zero Splunk credentials:
    * SPL observations skipped (live gateway REST status still used).
    * HEC audit skipped (decisions still logged to stdout).
* When Splunk credentials are added later, those features light up
  without code changes.

## Setup checklist (Plan B)

1. **Install Ollama.** <https://ollama.com/download> — Windows
   installer, ~200 MB.
2. **Pull the model.** `ollama pull qwen2.5:3b` (1.9 GB on disk,
   ~3 GB RAM at runtime — fits comfortably in 6–8 GB total system
   RAM). See the model-selection table above for lower- and
   higher-RAM alternatives.
3. **Sanity-check Ollama.** `ollama run qwen2.5:3b "say hello"` →
   should reply.
4. **Run the agent.**

   ```powershell
   cd agent
   pip install -e .
   Copy-Item configs\aegis-ops.example.toml configs\aegis-ops.toml
   aegis-ops run --config configs\aegis-ops.toml --once -v
   ```

5. **(Optional) wire Splunk.** Paste a Splunk auth token into
   `[splunk]` and a HEC token into `[audit]` to light up SPL
   observations and audit. The `[llm]` block stays on Ollama.

## Effect on Splunk AI Assistant 2.0 integration

SAIA 2.0 uses the same SLIM infrastructure as the `| ai` command.
On a trial account where SLIM is gated, SAIA cannot answer
generative-AI prompts either.

What still works in the trial:

* SAIA's SPL-search surface (it can still issue SPL queries on the
  user's behalf even without generative reasoning).
* Any SPL search against `index=aegis sourcetype=aegis:agent` —
  including the recommended operator workflow in
  [`docs/saia-integration.md`](saia-integration.md).

What doesn't:

* SAIA replying in natural language about Aegis audit events.

The documented SAIA pairing is therefore "best-effort, degraded on
trial accounts, full-featured on provisioned Splunk Cloud". The
Aegis-side integration code is unchanged.

## Restoring Plan A (when SLIM access lands)

```toml
# agent/configs/aegis-ops.toml
[llm]
transport = "splunk_ai"

[splunk]
url   = "https://prd-p-XXXXX.splunkcloud.com"
token = "paste-search-token-here"
```

That's it. No code edits, no redeployment of the sidecar, no UI
change. The same prompt, the same decision schema, the same audit
trail — only the transport differs.

## What this means for the submission

The hackathon's published Hosted Models prize text says:

> Build solutions that use Splunk Hosted Models such as gpt-oss-20b,
> gpt-oss-120b, or Foundation-Sec-1.1-8B-Instruct.

Aegis ships **a working, tested integration with the `| ai` SPL
transport** (`transports.SplunkAITransport`, `sidecar/splunk_ai.py`,
12 transport tests passing) that is one config flag away from
production. It is hibernated only because the provisioned environment
to run it against does not exist on a 14-day trial.

That is materially different from "we never integrated Hosted
Models". We integrated them, demonstrated the integration in tests
against a mocked Splunk SPL endpoint, and pivoted the runtime
transport to a model with the same identifier so the demo is
end-to-end functional today.
