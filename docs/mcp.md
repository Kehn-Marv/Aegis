# MCP integration — Aegis on **both sides** of the protocol

Aegis is intentionally bidirectional with respect to the Model Context
Protocol:

| Direction | What it means | Endpoint | Purpose |
|---|---|---|---|
| **Aegis as MCP server** | External AI agents (Cursor, Claude Desktop, the official Splunk MCP TA, etc.) can hold conversation with Aegis's `Control` plane and call its tools | `http://127.0.0.1:7321/mcp` | Lets a human or third-party agent operate the edge gateway by natural language |
| **AegisOps Agent as MCP client** | Our own autonomous agent talks to the **official Splunk MCP Server** (`splunk_run_query`) instead of the raw `/services/search/jobs/oneshot` REST endpoint | `https://<splunk-host>:8089/services/mcp` | Every observational call traverses the same MCP control plane judges will be auditing; full traffic visible in `index=_internal sourcetype=mcpjson "tools/call"` |

The two directions are completely independent — you can enable either,
both, or neither at deployment time. The narrative for the **Best Use
of Splunk MCP Server** prize relies on *both*: Aegis publishes
gateway-control tools via MCP for upstream AI agents to call, *and*
AegisOps consumes Splunk's official MCP tools to ground its reasoning.

## Aegis MCP Server — tools published

The Aegis daemon hosts a Model Context Protocol server that exposes five
tools any MCP-aware AI agent can call:

| Tool          | Description                                                        |
|---------------|--------------------------------------------------------------------|
| `status`      | Live snapshot: queue depth, dedup ratio, online flag, uptime, etc. |
| `reset`       | Clear the priority queue and in-memory dedup counters              |
| `diagnostic`  | Enable verbose tracing at the edge for N seconds                   |
| `override`    | Disable compression and stream raw logs to HEC for N seconds       |
| `replay_raw`  | Re-emit buffered raw events for a given unix-time window           |

Two transports are supported. **Use the HTTP transport** for the demo —
it's the one that lets your AI agent control the *running* daemon with
its live `Control` state. The stdio transport spawns a fresh process per
session and is only useful for smoke-testing.

## HTTP transport (recommended)

Start the daemon normally (the HTTP MCP server binds automatically at the
address in `[mcp]` of `configs/aegis.toml`, default `127.0.0.1:7321`):

```powershell
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
Aegis gateway?"* — Cursor will call the `status` tool and show you the
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

### Why this matters for the hackathon

The **Best Use of Splunk MCP Server** prize text says:

> Awarded to the team that most effectively leverages the Splunk MCP
> Server to build intelligent, agent-driven experiences. This prize
> recognizes solutions that seamlessly connect AI agents to Splunk
> data, enabling powerful workflows such as automated investigation,
> contextual insights and real-time decision making. Judges will look
> for creative implementations that showcase how MCP can orchestrate
> meaningful actions across observability, security, platform and
> developer use cases.

Aegis hits this three different ways:

1. **AegisOps Agent → Splunk MCP** (this section). Every observation
   the autonomous agent makes is an MCP `tools/call`, with full
   guardrail enforcement from the Splunk MCP Server (1-minute search
   timeout, 1000-event cap, etc.).
2. **External AI agents → Aegis MCP** (sections above). Cursor /
   Claude Desktop / any MCP client can flip edge-gateway switches via
   Aegis's five tools.
3. **Both endpoints in a single Cursor/Claude session.** The chat-side
   orchestration block below shows how a single LLM holds tools from
   *both* Aegis and Splunk in one context — the canonical
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
  with TLS + bearer-token auth — the `rmcp` SDK ships an example of the
  middleware pattern.
* **`--mcp-only` (stdio)** uses a fresh `Control` per connection, so
  counters always read zero in that mode. Use the HTTP transport for any
  real demo.
