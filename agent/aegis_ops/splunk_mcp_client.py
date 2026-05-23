"""Splunk MCP Server client.

Speaks **MCP JSON-RPC 2.0** (protocol revision 2025-03-26) to either of:

* Splunk's official **Splunk MCP Server for the Splunk Platform** v1.1+
  (`splunk_*`-prefixed tools, served at `/services/mcp` on Splunk
  management port 8089).
* The community/Cisco-DevNet **Splunk-MCP-Server-official** (same
  endpoint, slightly different tool naming — `run_splunk_query` instead
  of `splunk_run_query`).

We auto-detect which is live by calling `tools/list` once on first use
and remembering the first tool whose name matches a search-style
pattern (`splunk_run_query` -> `run_splunk_query` -> `search_oneshot`
-> `search_splunk`). The `tool_name` config field overrides detection.

Why this exists for Aegis: when present, the AegisOps agent uses this
client instead of the raw `/services/search/jobs/oneshot` REST surface,
so *every observational call by the agent traverses the same MCP
control plane that any other AI agent would use*. That makes the
"Best Use of Splunk MCP Server" prize claim concrete and demoable:
toggle one flag in `aegis-ops.toml` and watch the agent's traffic
appear in `index=_internal sourcetype=mcpjson "tools/call"`.

Transport:
    The Splunk MCP Server supports `HTTP+SSE` (recommended) and a
    `streamable_http` variant. For the agent loop we use plain
    request/response JSON-RPC over HTTPS POST — the SSE channel is
    only required for server-pushed notifications, which we don't
    consume. Single-shot POSTs are what `tools/call` always returns,
    so this works for both server flavours.
"""

from __future__ import annotations

import json
import logging
from typing import Any

import httpx

log = logging.getLogger("aegis_ops.splunk_mcp")

PROTOCOL_VERSION = "2025-03-26"

# Order of preference when auto-detecting the search tool.
_SEARCH_TOOL_PREFERENCES = (
    "splunk_run_query",
    "run_splunk_query",
    "search_oneshot",
    "search_splunk",
)


class SplunkMcpError(RuntimeError):
    """Raised when the MCP server returns a JSON-RPC error."""


class SplunkMcpClient:
    def __init__(
        self,
        endpoint: str,
        token: str,
        verify_tls: bool = True,
        timeout: float = 30.0,
        tool_name: str | None = None,
        client_name: str = "aegis-ops",
        client_version: str = "0.1.0",
    ):
        """
        Args:
            endpoint:    full URL to the MCP server, e.g.
                         `https://splunk.example.com:8089/services/mcp`.
            token:       Splunk auth token (same one used by REST).
            verify_tls:  TLS verification; safe to disable on a
                         self-signed dev Splunk.
            timeout:     per-request HTTP timeout in seconds.
            tool_name:   name of the SPL-execution tool to call. If
                         `None`, the client introspects via `tools/list`
                         on first use and picks the first matching name
                         from `_SEARCH_TOOL_PREFERENCES`.
            client_name/version: MCP `clientInfo` shipped in `initialize`.
        """
        self.endpoint = endpoint
        self._client = httpx.AsyncClient(
            verify=verify_tls,
            timeout=timeout,
            headers={
                "Authorization": f"Bearer {token}",
                "Content-Type": "application/json",
                "Accept": "application/json, text/event-stream",
            },
        )
        self._tool_name_override = tool_name
        self._resolved_tool_name: str | None = None
        self._initialized = False
        self._client_name = client_name
        self._client_version = client_version
        self._next_id = 0

    async def close(self) -> None:
        await self._client.aclose()

    # ------------------------------------------------------------------
    # MCP plumbing
    # ------------------------------------------------------------------

    def _rpc_id(self) -> int:
        self._next_id += 1
        return self._next_id

    async def _rpc(self, method: str, params: dict | None = None) -> Any:
        body = {
            "jsonrpc": "2.0",
            "id": self._rpc_id(),
            "method": method,
        }
        if params is not None:
            body["params"] = params
        r = await self._client.post(self.endpoint, content=json.dumps(body))
        r.raise_for_status()
        # MCP servers may stream as SSE; for tools/* we treat the body as
        # JSON-RPC envelope. If the body begins with `data:` (SSE frame),
        # strip it.
        text = r.text.strip()
        if text.startswith("data:"):
            text = text.split("data:", 1)[1].strip()
        try:
            envelope = json.loads(text)
        except json.JSONDecodeError as exc:
            log.warning("MCP non-JSON reply (truncated): %s", text[:300])
            raise SplunkMcpError(f"non-JSON MCP reply: {exc}") from exc
        if "error" in envelope:
            err = envelope["error"]
            raise SplunkMcpError(
                f"MCP {method} failed: {err.get('code')} {err.get('message')}"
            )
        return envelope.get("result")

    async def _notify(self, method: str, params: dict | None = None) -> None:
        body = {"jsonrpc": "2.0", "method": method}
        if params is not None:
            body["params"] = params
        r = await self._client.post(self.endpoint, content=json.dumps(body))
        if r.status_code >= 400:
            log.warning(
                "MCP notification %s rejected: %d %s",
                method,
                r.status_code,
                r.text[:200],
            )

    async def _ensure_initialized(self) -> None:
        if self._initialized:
            return
        result = await self._rpc(
            "initialize",
            params={
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": self._client_name,
                    "version": self._client_version,
                },
            },
        )
        log.info(
            "MCP server: %s v%s (proto=%s)",
            (result or {}).get("serverInfo", {}).get("name", "?"),
            (result or {}).get("serverInfo", {}).get("version", "?"),
            (result or {}).get("protocolVersion", "?"),
        )
        await self._notify("notifications/initialized")
        self._initialized = True

    async def _resolve_search_tool(self) -> str:
        if self._resolved_tool_name is not None:
            return self._resolved_tool_name
        if self._tool_name_override:
            self._resolved_tool_name = self._tool_name_override
            log.info("MCP search tool: %s (from config)", self._resolved_tool_name)
            return self._resolved_tool_name
        result = await self._rpc("tools/list")
        names = [t.get("name") for t in (result or {}).get("tools", []) if t.get("name")]
        for preferred in _SEARCH_TOOL_PREFERENCES:
            if preferred in names:
                self._resolved_tool_name = preferred
                log.info("MCP search tool auto-detected: %s", preferred)
                return preferred
        raise SplunkMcpError(
            f"No known SPL-execution tool found on MCP server "
            f"(looked for {_SEARCH_TOOL_PREFERENCES!r}; saw {names!r})"
        )

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    async def list_tools(self) -> list[dict]:
        """Return the server's full tool catalog."""
        await self._ensure_initialized()
        result = await self._rpc("tools/list")
        return list((result or {}).get("tools", []))

    async def search(
        self,
        spl: str,
        earliest: str = "-5m",
        latest: str = "now",
        max_results: int = 1000,
    ) -> list[dict]:
        """Run an SPL search via MCP and return result rows.

        Mirrors `SplunkClient.oneshot()` so it can be used as a drop-in.
        """
        await self._ensure_initialized()
        tool = await self._resolve_search_tool()

        arguments = {
            "query": spl if spl.strip().startswith("|") or spl.strip().startswith("search") else f"search {spl}",
            "earliest_time": earliest,
            "latest_time": latest,
            "max_results": max_results,
        }
        result = await self._rpc(
            "tools/call",
            params={"name": tool, "arguments": arguments},
        )
        return _parse_tool_call_result(result, tool=tool)


def _parse_tool_call_result(result: Any, *, tool: str) -> list[dict]:
    """Turn an MCP `tools/call` result into a list of result rows.

    MCP `tools/call` returns:

        {
          "content": [
            {"type": "text", "text": "..."},
            ...
          ],
          "isError": false
        }

    Different Splunk MCP servers serialize the search rows differently
    inside the text payload:

    * **splunk-mcp-server2** returns markdown / JSON / CSV / summary
      depending on the configured output format. The default is
      markdown — useless for us. We parse JSON when we see a `[` or
      `{`, else we return one row per non-empty text block.
    * **Cisco-DevNet `Splunk-MCP-Server-official`** returns Splunk
      result objects already JSON-encoded.
    * **Splunk Cloud `splunk_run_query`** returns a JSON array of
      result objects in `result.content[0].text`.
    """
    if not result:
        return []
    if isinstance(result, dict) and result.get("isError"):
        log.warning("MCP %s returned isError=true: %s", tool, result)
        return []

    content = (result or {}).get("content") if isinstance(result, dict) else None
    if not content:
        return []

    rows: list[dict] = []
    for block in content:
        if not isinstance(block, dict):
            continue
        text = block.get("text")
        if not text:
            continue
        stripped = text.lstrip()
        if stripped.startswith("[") or stripped.startswith("{"):
            try:
                parsed = json.loads(text)
            except json.JSONDecodeError:
                rows.append({"_raw": text})
                continue
            if isinstance(parsed, list):
                for item in parsed:
                    if isinstance(item, dict):
                        rows.append(item)
                    else:
                        rows.append({"value": item})
            elif isinstance(parsed, dict):
                if "results" in parsed and isinstance(parsed["results"], list):
                    rows.extend(r for r in parsed["results"] if isinstance(r, dict))
                else:
                    rows.append(parsed)
        else:
            rows.append({"_raw": text})
    return rows
