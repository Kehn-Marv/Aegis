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
