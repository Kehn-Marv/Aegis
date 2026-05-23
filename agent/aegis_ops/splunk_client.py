"""Splunk REST API client.

Used for:
  * **Observational search** — SPL queries against `index=aegis` to
    extract top signatures, anomaly counts, classifier breakdowns,
    rolling trends. Hits `/services/search/jobs/oneshot`.
  * **Hosted-model inference** — POSTs an SPL string containing the
    AI Toolkit's `| ai` command to invoke a Splunk Hosted Model.
    Same one-shot endpoint; the result row's text *is* the model reply.
  * **HEC audit** — ships `DecisionRecord` events to HEC under
    sourcetype `aegis:agent`.

`oneshot` is synchronous from the caller's POV (no `sid` polling),
which keeps the agent loop simple. For long-running searches we'd
switch to async `jobs` + polling later.
"""

from __future__ import annotations

import json
import logging

import httpx

from .splunk_mcp_client import SplunkMcpClient

log = logging.getLogger("aegis_ops.splunk")


class SplunkClient:
    """REST + MCP wrapper for Splunk.

    When an `SplunkMcpClient` is passed in, **all observational SPL
    queries** go through MCP `tools/call` against the official Splunk
    MCP Server instead of the raw `/services/search/jobs/oneshot` REST
    endpoint. The HEC audit path is unaffected (HEC is not part of the
    MCP surface; it stays REST).

    The dual surface means:

    * Operators can demo Aegis with raw REST today (zero MCP setup).
    * Once `[splunk.mcp].enabled = true` and the Splunk MCP Server app
      is installed, the *same* agent loop becomes an MCP client of
      `splunk_run_query` -- everything visible in `index=_internal
      sourcetype=mcpjson "tools/call"` for audit.
    """

    def __init__(
        self,
        url: str,
        token: str,
        verify_tls: bool = True,
        timeout: float = 30.0,
        mcp: SplunkMcpClient | None = None,
    ):
        self.url = url.rstrip("/")
        self._client = httpx.AsyncClient(
            verify=verify_tls,
            timeout=timeout,
            headers={"Authorization": f"Bearer {token}"},
        )
        self.mcp = mcp

    async def close(self) -> None:
        await self._client.aclose()
        if self.mcp is not None:
            await self.mcp.close()

    @property
    def transport_label(self) -> str:
        return "mcp" if self.mcp is not None else "rest"

    # ------------------------------------------------------------------
    # Observational SPL
    # ------------------------------------------------------------------

    async def oneshot(self, spl: str, earliest: str = "-5m", latest: str = "now") -> list[dict]:
        """Run an SPL search and return the parsed result rows.

        Routes via MCP `tools/call` when an `SplunkMcpClient` is wired
        in; otherwise falls back to `/services/search/jobs/oneshot`.
        """
        if self.mcp is not None:
            try:
                return await self.mcp.search(spl, earliest=earliest, latest=latest)
            except Exception as exc:
                log.warning(
                    "MCP search failed (%s); falling back to REST oneshot.",
                    exc,
                )
                # Fall through to REST so a misconfigured MCP server
                # can't take the agent loop down.

        data = {
            "search": spl if spl.strip().startswith("search") or spl.strip().startswith("|") else f"search {spl}",
            "output_mode": "json",
            "earliest_time": earliest,
            "latest_time": latest,
            "exec_mode": "oneshot",
        }
        r = await self._client.post(
            f"{self.url}/services/search/jobs/oneshot",
            data=data,
        )
        if r.status_code != 200:
            log.warning(
                "splunk oneshot returned %d: %s",
                r.status_code,
                r.text[:300],
            )
        r.raise_for_status()
        try:
            payload = r.json()
        except Exception:
            log.warning("splunk oneshot returned non-JSON body: %s", r.text[:300])
            return []
        return payload.get("results", [])


class HecClient:
    """Minimal Splunk HEC client for shipping audit events."""

    def __init__(
        self,
        endpoint: str,
        token: str,
        verify_tls: bool = False,
        timeout: float = 10.0,
    ):
        self.endpoint = endpoint
        self._client = httpx.AsyncClient(
            verify=verify_tls,
            timeout=timeout,
            headers={
                "Authorization": f"Splunk {token}",
                "Content-Type": "application/json",
            },
        )

    async def close(self) -> None:
        await self._client.aclose()

    async def send(self, event: dict) -> None:
        r = await self._client.post(self.endpoint, content=json.dumps(event))
        if r.status_code >= 300:
            log.warning("HEC rejected event: %d %s", r.status_code, r.text[:300])
        r.raise_for_status()
