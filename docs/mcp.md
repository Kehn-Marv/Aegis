# Aegis MCP Server — client integration

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
