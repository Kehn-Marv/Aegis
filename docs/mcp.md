# MCP integration  -  Aegis on **both sides** of the protocol

Aegis is intentionally bidirectional with respect to the Model Context
Protocol:

| Direction | What it means | Endpoint | Purpose |
|---|---|---|---|
| **Aegis as MCP server** | External AI agents (Cursor, Claude Desktop, the official Splunk MCP TA, etc.) can hold conversation with Aegis's `Control` plane and call its tools | `http://127.0.0.1:7321/mcp` | Lets a human or third-party agent operate the edge gateway by natural language |
| **AegisOps Agent as MCP client** | Our own autonomous agent talks to the **official Splunk MCP Server** (`splunk_run_query`) instead of the raw `/services/search/jobs/oneshot` REST endpoint | `https://<splunk-host>:8089/services/mcp` | Every observational call traverses the same MCP control plane judges will be auditing; full traffic visible in `index=_internal sourcetype=mcpjson "tools/call"` |

The two directions are completely independent  -  you can enable either,
both, or neither at deployment time.

## Aegis MCP Server  -  tools published

The Aegis daemon hosts a Model Context Protocol server that exposes
eight tools any MCP-aware AI agent can call:

| Tool                 | Description                                                                  |
|----------------------|------------------------------------------------------------------------------|
| `status`             | Live snapshot: dedup ratio, queue depth, health state (green/orange/red), latest decision card |
| `latest_decision`    | The current decision card the engineer should be looking at, or `null`       |
| `recent_incidents`   | Top-N fingerprints from Aegis's incident memory                              |
| `resolve_incident`   | Attach a cause + fix resolution card to an incident in memory                |
| `acknowledge`        | Mark the current decision as 'I'm on it' (no production side effects)        |
| `reset`              | Clear the priority queue, dedup counters, and the current decision card     |
| `diagnostic`         | Enable verbose tracing at the edge for N seconds (bounded window)            |
| `override`           | Disable compression and stream raw logs to HEC for N seconds (bounded)       |
| `replay_raw`         | Re-emit buffered raw events for a given unix-time window (currently a stub)  |

The three highlighted tools (`latest_decision`, `recent_incidents`,
`resolve_incident`) are new in Aegis v0.2 and make the agent a real
participant in the incident-memory loop. An AI agent can read the
current card, look up past matches, attach a fix the engineer dictated
in chat, and Aegis remembers  -  same flow as the React UI uses.

Two transports are supported. **Use the HTTP transport** for the demo  - 
it's the one that lets your AI agent control the *running* daemon with
its live `Control` state. The stdio transport spawns a fresh process per
session and is only useful for smoke-testing.

## HTTP transport (recommended)

Start the daemon. The MCP server binds automatically at the address in
`[mcp]` of the config (default `127.0.0.1:7321`):

```powershell
# Docker (easiest):
docker compose up --build
# MCP endpoint → http://localhost:7321/mcp

# Or from source:
cargo run --bin aegis-daemon
# Aegis logs:  MCP HTTP listening at 127.0.0.1:7321/mcp
```

Verify the endpoint is reachable:

```powershell
curl.exe -X POST `
    -H "Content-Type: application/json" `
    -H "Accept: application/json, text/event-stream" `
    -d '{"jsonrpc":"2.0","method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"smoke","version":"0.0.1"}},"id":1}' `
    http://127.0.0.1:7321/mcp
```

You should get a JSON-RPC `initialize` response with the server name
`aegis-mcp` and a list of capabilities.

### Cursor

Add to `%USERPROFILE%\.cursor\mcp.json` (or your workspace's
`.cursor/mcp.json`):

```json
{
  "mcpServers": {
    "aegis": {
      "url": "http://127.0.0.1:7321/mcp"
    }
  }
}
```

Then open Cursor → Settings → MCP and you should see `aegis` listed with
its 5 tools. Try asking the chat: *"What is the current status of the
Aegis gateway?"*  -  Cursor will call the `status` tool and show you the
live numbers.

### Claude Desktop

Add to `%APPDATA%\Claude\claude_desktop_config.json` (Windows) or
`~/Library/Application Support/Claude/claude_desktop_config.json` (macOS):

```json
{
  "mcpServers": {
    "aegis": {
      "url": "http://127.0.0.1:7321/mcp"
    }
  }
}
```

Restart Claude Desktop. The Aegis tools will appear in the tool picker.

### Orchestrating with the Splunk MCP Server

The whole point of MCP is that one AI agent can hold tools from *multiple*
servers in the same conversation. Register Aegis alongside the official
Splunk MCP Server so a single chat session can run SPL searches *and*
flip edge-gateway switches:

```json
{
  "mcpServers": {
    "aegis": {
      "url": "http://127.0.0.1:7321/mcp"
    },
    "splunk": {
      "url": "https://your-splunk-host:8089/services/mcp",
      "headers": {
        "Authorization": "Splunk YOUR-SPLUNK-TOKEN"
      }
    }
  }
}
```

Then prompts like *"Find the top 3 signatures by collapsed count in the
last hour and call aegis.override(seconds=30) for any service that's
seeing >10x its baseline"* become a single agentic loop the LLM can
execute end-to-end.

## stdio transport (subprocess mode)

For MCP clients that prefer to spawn the server as a subprocess (some
debuggers and inspectors do), build the release binary and point them at
it:

```json
{
  "mcpServers": {
    "aegis-stdio": {
      "command": "C:\\Users\\you\\Desktop\\splunk\\target\\release\\aegis-daemon.exe",
      "args": ["--mcp-only", "--config", "configs/aegis.toml"]
    }
  }
}
```

Note: this transport gives the MCP server its own `Control` instance with
zero counters. Useful for testing the tool surface in isolation; not
useful for inspecting a running daemon.

## AegisOps Agent as MCP **client** of the Splunk MCP Server

The autonomous AegisOps agent (`agent/aegis_ops/`) can be configured to
route every observational SPL call through the **official Splunk MCP
Server** instead of the raw REST `oneshot` endpoint:

```toml
# agent/configs/aegis-ops.toml
[splunk]
url   = "https://localhost:8089"
token = "your-splunk-auth-token"

[splunk.mcp]
enabled  = true
endpoint = "https://localhost:8089/services/mcp"
# tool_name = ""   # leave empty for auto-detect
```

On startup the agent logs:

```
INFO MCP server: Splunk_MCP_Server v1.1.0 (proto=2025-03-26)
INFO MCP search tool auto-detected: splunk_run_query
INFO AegisOps starting: ... splunk=on/mcp ...
```

…and every reasoning tick now produces a `tools/call` traffic line
visible in Splunk's own audit index:

```spl
index=_internal sourcetype=mcpjson "tools/call" "splunk_run_query"
| stats count by source
```

### How Aegis uses MCP on both sides

1. **AegisOps Agent → Splunk MCP** (this section). Every observation
   the autonomous agent makes is an MCP `tools/call`, with full
   guardrail enforcement from the Splunk MCP Server (1-minute search
   timeout, 1000-event cap, etc.).
2. **External AI agents → Aegis MCP** (sections above). Cursor /
   Claude Desktop / any MCP client can flip edge-gateway switches via
   Aegis's tools.
3. **Both endpoints in a single Cursor/Claude session.** The chat-side
   orchestration block above shows how a single LLM holds tools from
   *both* Aegis and Splunk in one context  -  the canonical
   multi-server MCP demo.

### Implementation notes

* **Auto-detection** of the search tool. The client calls `tools/list`
  on first use and picks the first name matching
  `splunk_run_query` → `run_splunk_query` → `search_oneshot` →
  `search_splunk`. Override via `[splunk.mcp].tool_name`.
* **Graceful REST fallback.** If the MCP call fails (network blip,
  guardrail rejection), the same SPL is retried via REST `oneshot` so
  a misconfigured MCP server can't take down the agent loop.
  Cleanest demo path: keep MCP enabled; cut Splunk; watch the agent
  cleanly degrade in `index=aegis sourcetype=aegis:agent`.
* **JSON-RPC implementation** in
  [`agent/aegis_ops/splunk_mcp_client.py`](../agent/aegis_ops/splunk_mcp_client.py).
  Speaks protocol revision `2025-03-26`. Handles both plain JSON and
  SSE-framed responses. Tested with the result-parser unit tests in
  `agent/tests/test_splunk_mcp_client.py`.

## Limitations / TODO

* **`replay_raw`** is a stub. The current queue acks events immediately
  after a successful HEC send, so there's nothing to replay. A future
  iteration will add a separate history table with a configurable TTL
  that `replay_raw` can read from.
* **No authentication on the HTTP transport.** The default bind is
  `127.0.0.1:7321`, which limits exposure to localhost. If you need to
  expose this beyond the local machine, put it behind a reverse proxy
  with TLS + bearer-token auth  -  the `rmcp` SDK ships an example of the
  middleware pattern.
* **`--mcp-only` (stdio)** uses a fresh `Control` per connection, so
  counters always read zero in that mode. Use the HTTP transport for any
  real demo.
