# Troubleshooting

Symptom → fix reference for Path B (Splunk + dashboard), Path C
(AegisOps agent), and shared infrastructure. Setup steps live in
[`README.md`](README.md).

---

## Rust / daemon

| Symptom | Fix |
|---|---|
| `cargo build` fails with a linker / `link.exe` error on Windows | Install MSVC Build Tools (the cargo error message has the link). |
| Daemon prints `bind tcp listener at 127.0.0.1:5140 ... Os { code: 10048 ... }` | Another process is already on port 5140 (often a previous Aegis daemon you forgot to kill). `Get-Process aegis-daemon \| Stop-Process -Force`, then restart. |
| Daemon prints `bind aegis http listener at 127.0.0.1:7321 ... already in use` | Same, but for the MCP/REST port. Edit `mcp.http_listen` in your config or kill the stale daemon. |
| `cargo run -- --check-hec` returns `HEC rejected events: 401` | Bad or disabled HEC token. Re-issue the token in Splunk Web and update `configs/aegis.toml`. |
| `cargo run -- --check-hec` returns a TLS error | Self-signed cert. Set `verify_tls = false` in `[hec]`. |
| Daemon logs `self-metric emit failed` repeatedly | Splunk or HEC was temporarily unreachable (restart, sleep, network). The daemon keeps running — verify with `Invoke-RestMethod http://127.0.0.1:7321/api/status` (`online: true` = healthy). Fix Splunk/HEC: `cargo run --release --bin aegis-daemon -- --check-hec` should print `HEC ping accepted`. Re-run the spammer to refresh dashboard data. Optional: restart the daemon after Splunk is back up. |
| `Invoke-RestMethod http://127.0.0.1:7321/api/status` fails | Daemon not running. Start Terminal 2: `cargo run --release --bin aegis-daemon` from the repo root; wait for `tcp ingest listening` before retrying. |

---

## Splunk search / HEC / dashboard

| Symptom | Fix |
|---|---|
| Splunk search returns nothing even though daemon says `HEC batch delivered` | Run the search in **Search & Reporting** (`http://localhost:8000`), not the HEC URL on port 8088. Check the *index* in your search matches `index=aegis` (or whatever you set). Check the time range covers when events landed. |
| `index=aegis` returns zero events after running the B4 log spammer | 1) Confirm Terminals 1 and 2 (sidecar + daemon) are still running. 2) Re-run the spammer while both are up: `python demo\log_spammer.py --target tcp://127.0.0.1:5140 --pattern crashloop --rate 200 --duration 60`. 3) Wait ~60 seconds after it finishes so dedup metric windows can flush. 4) In **Search & Reporting**, run `index=aegis` with time range *Last 15 minutes* and click **Search**. |
| Dashboard **AI classifier verdict** panel is empty | The sidecar only classifies **new** traffic. Follow [B4a–B4b in README](README.md#b4-run-the-live-pipeline): 1) Terminal 1 — `python -m aegis_sidecar.server` (`Uvicorn running on http://127.0.0.1:8765`). 2) Terminal 2 — confirm daemon with `Invoke-RestMethod http://127.0.0.1:7321/api/status`. 3) Re-run the spammer while both are up. 4) Wait 60s. 5) Search `index=aegis sourcetype=aegis:metric "classification.label"=*` in **Search & Reporting**, then refresh the dashboard. |
| Dashboard CDTSM panels show `Unknown search command 'apply'` | AI Toolkit not installed. Complete [B3.6 in README](README.md#b36-install-splunk-ai-toolkit-required--powers-cdtsm-forecast-panels), restart Splunk, re-open the dashboard. On Cloud, panels also need ~15 min of `aegis:selfmetric` data. |
| `CDTSM: Failed to retrieve tenant info: HTTP 404 Not Found` | **Expected on local Splunk Enterprise.** CDTSM is Splunk-Hosted (SLIM/Cloud only). Path B still works — 9 of 11 dashboard panels populate. See [B3.6b in README](README.md#b36b-smoke-test-cdtsm-splunk-cloud-only) and [`docs/splunk-blocker.md`](docs/splunk-blocker.md). |

---

## Splunk AI Toolkit / `| ai`

| Symptom | Fix |
|---|---|
| PSC install fails (`Winsock 10054`, `Internal Server Error`, `Package is too large`) | PSC (~800 MB) must use **CLI**, not Splunk Web. Download the `.tgz` from [Splunkbase](https://splunkbase.splunk.com/app/2883), then while Splunk is **running**: `splunk install app path\to\file.tgz -update 1 -auth user:pass` → `splunk restart`. See [B3.6 in README](README.md#b36-install-splunk-ai-toolkit-required--powers-cdtsm-forecast-panels). |
| `install app` says `splunkd is unreachable` | Splunk is stopped — run `splunk start` first, wait for Splunk Web, then `install app`. |
| AITK web Install fails | AITK is only ~30 MB — retry **Apps → Find More Apps → Splunk AI Toolkit → Install**. If it keeps failing, download from [Splunkbase](https://splunkbase.splunk.com/app/2890) and use the same CLI `install app` path as PSC. |
| `Error in 'ai' command: User does not have permission` | Grant **`apply_ai_commander_command`** on your Splunk role: **Settings → Roles → admin → Capabilities**. See [B3.6a in README](README.md#b36a-grant-ai-command-permission-one-time). |
| `Error in 'ai' command: No default LLM configuration found` | You have not configured a default LLM in AITK Connection Management. For Path C, set up Ollama — see [B3.6c in README](README.md#b36c-optional--smoke-test-ai-path-c--ollama) and [`docs/aitk-ollama.md`](docs/aitk-ollama.md). Unrelated to CDTSM. |

---

## Sidecar / UI

| Symptom | Fix |
|---|---|
| `npm install` in `ui/` is slow | First install is ~1 minute (82 packages). Subsequent runs are seconds. |
| Sidecar startup error: `ModuleNotFoundError: No module named 'aegis_sidecar'` | You're running `python server.py` directly. Use `pip install -e .` inside the `sidecar/` virtualenv, then `python -m aegis_sidecar.server`. |
| `pip install -e .` in `sidecar/` fails with `ConnectionResetError` while downloading `torch` | Transient network drop on the ~123 MB PyTorch wheel. Run `pip install torch --index-url https://download.pytorch.org/whl/cpu` first, then `pip install -e .` again — pip reuses cached packages. The `win_amd64` in the wheel name is correct for 64-bit Windows on Intel or AMD — it does not mean you need an AMD CPU. |
| Sidecar takes ~30 s on first `/classify` call | Lazy-loading the sentence-transformer model (~80 MB). Subsequent calls are sub-millisecond. |
| UI shows "UNREACHABLE" badge | Daemon isn't running, or its MCP/REST port differs from `7321`. Confirm the daemon log says `Control API at 127.0.0.1:7321/api/status`. |

---

## AegisOps agent (Path C)

| Symptom | Fix |
|---|---|
| Agent log: `CERTIFICATE_VERIFY_FAILED` on port **8089** | Set `[splunk] verify_tls = false` in `agent/configs/aegis-ops.toml` (same self-signed cert issue as HEC in Path B). |
| Agent log: `ollama call failed` / `ReadTimeout` / `conf=0.00 transport_returned_empty` | On **CPU-only** Ollama (`ollama ps` shows `100% CPU`), the full AegisOps prompt (~3.6 KB system prompt + Splunk observation JSON) with `qwen2.5:3b` takes **~5 minutes per gateway** — not the same as a quick `ollama run … "pong"`. Set `[llm.ollama] timeout_secs = 600` (or higher for two gateways). Warm the model first: `ollama run qwen2.5:3b "reply pong"`. Success looks like `conf=0.95` (not `conf=0.00`). For faster runs: `gemma2:2b` / `qwen2.5:1.5b`, or a GPU-backed Ollama host. |
| Agent shows `splunk=on, audit=on` but `decision=noop conf=0.00` | Ollama did not return a valid JSON decision in time — fix Ollama timeout above. Audit events may still land in Splunk (`sourcetype=aegis:agent`) with the noop fallback. |
| Two gateways: one succeeds, one times out | Ollama serializes requests on CPU. Both fire in parallel but the second waits in queue. Keep `timeout_secs = 600`; expect ~4–5 min total wall clock for two gateways when the model is warm. |
| Continuous loop feels like it should run every 15 s | `[agent].loop_interval_secs = 15` is a **minimum gap when ticks are fast**. If a tick takes 5 minutes, the next tick starts ~0.5 s after it finishes — not every 15 s. On CPU Ollama, use `--once` for demos or expect ~5 min per cycle. |
| `aegis-ops run --config ...` fails | Correct command is `aegis-ops --config configs\aegis-ops.toml --once -v` (no `run` subcommand). Run from `agent/` with venv activated. |
| Config edits lost after `Copy-Item` | `agent/configs/aegis-ops.toml` is **gitignored** (contains secrets). Copy from `aegis-ops.example.toml` once, then edit in place — do not re-copy over your tokens. |

---

## Secrets / git

| Symptom | Fix |
|---|---|
| Worried about committing tokens | Only `*.example.toml` files are tracked. Local configs (`configs/aegis.toml`, `configs/aegis-us-east.toml`, `configs/aegis-eu-west.toml`, `agent/configs/aegis-ops.toml`) are in [`.gitignore`](.gitignore). Run `git status` before pushing — none of those should appear as staged. |
| Accidentally committed a token | Rotate the token in Splunk immediately, remove the file from git history, and never reuse the leaked value. |
