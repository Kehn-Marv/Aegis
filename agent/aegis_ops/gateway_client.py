"""Client for the Aegis gateway's REST control API.

Calls /api/status (read) and /api/command (write). Maps cleanly onto the
five tools the daemon's MCP server also exposes; we use REST here
because the agent isn't an LLM and doesn't need MCP semantics.
"""

from __future__ import annotations

import httpx

from .models import GatewayStatus


class GatewayClient:
    def __init__(self, base_url: str, timeout: float = 10.0):
        self.base = base_url.rstrip("/")
        self._client = httpx.AsyncClient(timeout=timeout)

    async def close(self) -> None:
        await self._client.aclose()

    async def status(self) -> GatewayStatus:
        r = await self._client.get(f"{self.base}/api/status")
        r.raise_for_status()
        return GatewayStatus.model_validate(r.json())

    async def health(self) -> bool:
        try:
            r = await self._client.get(f"{self.base}/api/health")
            return r.status_code == 200
        except Exception:
            return False

    async def command(self, command: str, seconds: int | None = None) -> dict:
        body: dict = {"command": command}
        if seconds is not None:
            body["seconds"] = seconds
        r = await self._client.post(f"{self.base}/api/command", json=body)
        r.raise_for_status()
        return r.json()

    async def latest_decision(self) -> dict | None:
        """Return the current decision card, or `None` when the gateway is
        green (no active incident).

        Older Aegis gateways without `/api/decision` return 404 silently;
        the observer treats that as "no card".
        """
        r = await self._client.get(f"{self.base}/api/decision")
        if r.status_code == 404:
            return None
        r.raise_for_status()
        try:
            data = r.json()
        except Exception:
            return None
        if isinstance(data, dict) and data.get("kind") == "decision_card":
            return data
        return None

    async def recent_incidents(self, limit: int = 20) -> list[dict]:
        """List recent fingerprints from the gateway's incident memory."""
        r = await self._client.get(f"{self.base}/api/incidents", params={"limit": limit})
        if r.status_code == 404:
            return []
        r.raise_for_status()
        try:
            data = r.json()
        except Exception:
            return []
        return list(data.get("incidents", []) or [])
