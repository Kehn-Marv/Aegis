# Troubleshooting

Symptom → fix reference. Setup steps live in [`README.md`](README.md).

---

## Daemon / Rust

| Symptom | Fix |
|---|---|
| `cargo build` fails with a linker / `link.exe` error on Windows | Install MSVC Build Tools (the cargo error message has the link). |
| Daemon prints `bind tcp listener at 127.0.0.1:5140 ... already in use` | Another process is already on port 5140 (often a stale daemon). `Get-Process aegis-daemon \| Stop-Process -Force`, then restart. |
| Daemon prints `bind aegis http listener at 127.0.0.1:7321 ... already in use` | Same, but for the MCP/REST port. Edit `mcp.http_listen` in your config or kill the stale daemon. |
| `cargo run -- --check-hec` returns `HEC rejected events: 401` | Bad or disabled HEC token. Re-issue in Splunk Web and update `configs/aegis.toml`. |
| `cargo run -- --check-hec` returns a TLS error | Self-signed cert. Set `verify_tls = false` in `[hec]`. |
| Daemon logs `self-metric emit failed` repeatedly | Splunk or HEC briefly unreachable. The daemon keeps running; verify with `Invoke-RestMethod http://127.0.0.1:7321/api/status` (`online: true`). |
| `failed to open incident memory; MCP will run without past-incident recall` | The `[memory].path` directory isn't writable. Create `data/` and check permissions. |

---

## Decision card / causal chain

| Symptom | Fix |
|---|---|
| `cascade` pattern fires but the decision card stays green | The cascade overflows the configured causal window. Demo configs use `[causal].window_secs = 30` — make sure you're using `aegis.demo.toml` and that the cascade hasn't been customised to take longer than 30s. |
| Decision card shows the wrong service as root cause | The earliest service's first-fire fell out of the causal window before the chain triggered. Increase `[causal].window_secs`. Default in `aegis.example.toml` is 60s. |
| Stack-trace continuation lines (`  at db::…`) get tagged with their TCP source as a phantom "service" | Should be inherited automatically since v0.2 — `service::extract_full` walks per-source last-known-service. If you see this, file an issue with the continuation pattern. |
| `[CHAIN ...]` never fires even with 3 services breaking | Check `[causal].min_services` and `[causal].cooldown_secs`. Default min is 3; lower to 2 for small fleets. Cooldown suppresses re-emission for the same root cause — if you keep restarting the cascade, you need to wait or restart the daemon. |
| `[DECIDE state=green]` immediately after a chain | This is normal: after `[decision].idle_to_green_secs` of quiet (default 300s, demo 30s), Aegis auto-downshifts. The chain itself is still stored in incident memory. |

---

## Incident memory

| Symptom | Fix |
|---|---|
| `recent_incidents` MCP tool returns `{"incidents":[],"note":"memory store not attached"}` | The daemon's `--mcp-only` mode runs stdio without the store. Use the default HTTP mode (no flag) or `--mcp-http-only`. |
| `POST /api/incidents/{id}/resolve` returns `incident not found` | The id doesn't match anything in the store. List incidents first (`GET /api/incidents`) to copy the correct id. IDs are 16 hex characters. |
| Past incident memory file got too big | Each row is ~2 KB. At 10K incidents → ~20 MB. To prune: stop the daemon, open the SQLite file with any tool, `DELETE FROM incidents WHERE ts < strftime('%s','now','-90 days')`. Aegis will re-fingerprint anything that fires again. |
| Two daemons fight over the same memory file | SQLite WAL mode handles concurrent reads fine; writes serialise. If two regional gateways need shared memory, point both at the same path on a shared filesystem. For high-volume edges, run one Aegis per region. |

---

## Splunk search / HEC / dashboard

| Symptom | Fix |
|---|---|
| `index=aegis` returns nothing after a cascade | Wait ~60s for dedup windows to flush. Confirm sidecar + daemon are still running. The cascade pattern itself is ~10s. |
| Dashboard panels are empty | Time range may be too narrow. Set to *Last 15 minutes* and turn auto-refresh on. |
| Dashboard CDTSM panels show `Unknown search command 'apply'` | Install Splunk AI Toolkit from Splunkbase. See [`docs/cdtsm-forecast.md`](docs/cdtsm-forecast.md). |
| `CDTSM: Failed to retrieve tenant info: HTTP 404 Not Found` | **Expected on local Splunk Enterprise.** CDTSM is Splunk Cloud / SLIM only. The other panels still work. See [`docs/splunk-blocker.md`](docs/splunk-blocker.md). |

---

## Splunk AI Toolkit / `| ai`

| Symptom | Fix |
|---|---|
| PSC install fails (Winsock 10054, Internal Server Error) | PSC (~800 MB) needs CLI install, not Splunk Web. `splunk install app path\to\file.tgz -update 1 -auth user:pass`. |
| `Error in 'ai' command: User does not have permission` | Grant `apply_ai_commander_command` on your Splunk role: **Settings → Roles → admin → Capabilities**. |
| `Error in 'ai' command: No default LLM configuration found` | Configure an AITK LLM connection (Ollama is the easy path on local Enterprise). See [`docs/aitk-ollama.md`](docs/aitk-ollama.md). |

---

## Sidecar / UI

| Symptom | Fix |
|---|---|
| Sidecar `ModuleNotFoundError: aegis_sidecar` | You ran `python server.py` directly. Use `python -m aegis_sidecar.server` inside the venv. |
| `pip install -e .` in `sidecar/` fails with `ConnectionResetError` on torch | Transient network drop on the PyTorch wheel. Retry with `pip install torch --index-url https://download.pytorch.org/whl/cpu` first, then `pip install -e .`. |
| Sidecar takes ~30s on first `/classify` call | Lazy-loading the sentence-transformer (~80 MB). Subsequent calls are sub-millisecond. |
| UI shows "Gateway unreachable" | Daemon isn't running, or its MCP/REST port differs from `7321`. Daemon log should say `Control API at 127.0.0.1:7321/api/status`. |
| UI decision-card panel doesn't update | The poll runs every 2 s; if `/api/status` is reachable but always returns `decision: null`, you haven't fired a chain yet — run the `cascade` pattern. |

---

## AegisOps agent (Path C)

| Symptom | Fix |
|---|---|
| Agent log `CERTIFICATE_VERIFY_FAILED` on port 8089 | Set `[splunk] verify_tls = false` in `agent/configs/aegis-ops.toml`. |
| Agent log `ollama call failed` / `conf=0.00` | On CPU-only Ollama, the first call can take several minutes per gateway. Set `[llm.ollama] timeout_secs = 600` and warm the model first (`ollama run qwen2.5:3b "reply pong"`). |
| Agent two gateways: one succeeds, one times out | Ollama serializes requests on CPU. Expect ~5 min total wall clock for two gateways. |
| `aegis-ops run --config ...` fails | The CLI doesn't have a `run` subcommand — just `aegis-ops --config configs\aegis-ops.toml --once -v`. |
| Config edits lost after `Copy-Item` | `agent/configs/aegis-ops.toml` is **gitignored**. Copy from `aegis-ops.example.toml` once, then edit in place. |

---

## Secrets / git

| Symptom | Fix |
|---|---|
| Worried about committing tokens | Only `*.example.toml` files are tracked. Run `git status` before pushing — no real config should be staged. |
| Accidentally committed a token | Rotate immediately in Splunk Web, remove from git history, never reuse. |
