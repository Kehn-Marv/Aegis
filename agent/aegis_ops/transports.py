"""LLM transports for the reasoner.

The reasoner needs to send a prompt to a model and get text back. We
support two transports today, both behind the same `LLMTransport`
protocol so the reasoner doesn't care which one is active:

* **OllamaTransport** — POSTs `/api/chat` on a local Ollama server.
  Zero Splunk credentials. Default model is `qwen2.5:3b` (1.9 GB on
  disk, ~3 GB RAM, explicitly tuned by Alibaba for structured JSON
  output). When the transport is constructed with a `format_schema`,
  it uses Ollama's hard JSON-schema enforcement so even small 3B
  models reliably emit valid `Decision` objects.
* **SplunkAITransport** — runs SPL `| ai prompt=... provider=splunk_hosted
  model=gpt-oss-20b` via `/services/search/jobs/oneshot`. The original
  hackathon target; currently **hibernated** because the 14-day Splunk
  Cloud trial does not provision the SLIM API
  (see `docs/splunk-blocker.md`). All code paths are preserved so a
  single config flip re-activates this transport when SLIM is available.

Both transports return raw text; the reasoner does the JSON parsing
(when format_schema enforcement is active, the text *is* the JSON).
"""

from __future__ import annotations

import logging
from typing import Any, Protocol

import httpx

from .splunk_client import SplunkClient

log = logging.getLogger("aegis_ops.transport")


class LLMTransport(Protocol):
    """The reasoner only needs this surface area."""

    name: str

    async def call(self, prompt: str) -> str:
        """Return the model's reply as a plain string. Empty on failure."""
        ...

    async def close(self) -> None:
        ...


class OllamaTransport:
    """Local Ollama (`qwen2.5:3b`, `gemma2:2b`, ...) via /api/chat."""

    name = "ollama"

    def __init__(
        self,
        url: str = "http://127.0.0.1:11434",
        model: str = "qwen2.5:3b",
        timeout_secs: float = 60.0,
        format_schema: dict[str, Any] | None = None,
    ):
        self.url = url.rstrip("/")
        self.model = model
        self.format_schema = format_schema
        self._client = httpx.AsyncClient(timeout=timeout_secs)

    async def close(self) -> None:
        await self._client.aclose()

    async def call(self, prompt: str) -> str:
        body: dict[str, Any] = {
            "model": self.model,
            "stream": False,
            "options": {"temperature": 0.0},
            "messages": [{"role": "user", "content": prompt}],
        }
        # Ollama's `format` parameter accepts a JSON schema dict and
        # enforces it at decode time -- the model's reply is *guaranteed*
        # to be valid JSON matching the schema. Crucial for 3B models.
        if self.format_schema is not None:
            body["format"] = self.format_schema

        try:
            resp = await self._client.post(f"{self.url}/api/chat", json=body)
            resp.raise_for_status()
            data = resp.json()
        except Exception as exc:
            log.warning("ollama call failed (url=%s model=%s): %s", self.url, self.model, exc)
            return ""
        msg = data.get("message", {})
        if isinstance(msg, dict):
            content = msg.get("content")
            if isinstance(content, str):
                return content
        if isinstance(data.get("response"), str):
            return data["response"]
        return ""


class SplunkAITransport:
    """Splunk AI Toolkit `| ai` SPL transport. Hibernated until SLIM access lands."""

    name = "splunk_ai"

    def __init__(
        self,
        splunk: SplunkClient,
        provider: str = "splunk_hosted",
        model: str = "gpt-oss-20b",
    ):
        self.splunk = splunk
        self.provider = provider
        self.model = model

    async def close(self) -> None:
        # The agent owns the SplunkClient lifecycle; transport does not close it.
        return None

    async def call(self, prompt: str) -> str:
        spl = self._build_spl(prompt)
        try:
            rows = await self.splunk.oneshot(spl)
        except Exception as exc:
            log.warning("splunk |ai call failed: %s", exc)
            return ""
        return self._extract_text(rows)

    def _build_spl(self, prompt_text: str) -> str:
        escaped = prompt_text.replace("\\", "\\\\").replace('"', '\\"')
        return (
            f'| makeresults '
            f'| eval prompt="{escaped}" '
            f'| ai prompt=prompt provider={self.provider} model={self.model}'
        )

    @staticmethod
    def _extract_text(rows: list[dict]) -> str:
        if not rows:
            return ""
        row = rows[0]
        for key in ("ai_response", "response", "answer", "text", "ai"):
            value = row.get(key)
            if isinstance(value, str) and value.strip():
                return value
        return " ".join(str(v) for v in row.values() if isinstance(v, str))
