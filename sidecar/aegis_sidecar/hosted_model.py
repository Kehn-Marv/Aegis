"""Splunk Hosted Model adapter for log-line classification.

Two transports, tried in order when `classify()` runs:

1. **Splunk AI Toolkit (`| ai`)** — preferred. Set `AEGIS_SPLUNK_URL` and
   `AEGIS_SPLUNK_TOKEN`. This is the same integration path the AegisOps
   agent uses and satisfies the hackathon's Hosted Models prize track.
2. **OpenAI-compatible chat-completions** — fallback for local dev with
   vLLM, TGI, or Ollama. Set `AEGIS_HOSTED_MODEL_URL`.

If neither transport is configured, `classify()` returns None and the
classifier falls back to embedding-distance classification.
"""

from __future__ import annotations

import logging
import os
from typing import Literal

import httpx

from . import splunk_ai

log = logging.getLogger("aegis.hosted_model")

ClassifyLabel = Literal["anomaly", "routine", "unknown"]

_SYSTEM_PROMPT = (
    "You are an observability triage assistant. Classify the following log "
    "line as exactly one of: anomaly, routine, unknown. Respond with only "
    "the label, no other text."
)


def is_configured() -> bool:
    return splunk_ai.is_configured() or bool(os.environ.get("AEGIS_HOSTED_MODEL_URL"))


def transport() -> str | None:
    """Which hosted-model transport is active, for `/info` and dashboards."""
    if splunk_ai.is_configured():
        return "splunk_ai"
    if os.environ.get("AEGIS_HOSTED_MODEL_URL"):
        return "openai_compat"
    return None


def classify(line: str) -> tuple[ClassifyLabel | None, str | None]:
    """Classify a log line. Returns `(label, strategy)` or `(None, None)`."""
    if splunk_ai.is_configured():
        raw = splunk_ai.classify(line, system_prompt=_SYSTEM_PROMPT)
        if raw is not None:
            label = _normalise(raw)
            if label is not None:
                return label, "splunk_ai"

    label = _classify_openai_compat(line)
    if label is not None:
        return label, "openai_compat"
    return None, None


def _classify_openai_compat(line: str) -> ClassifyLabel | None:
    url = os.environ.get("AEGIS_HOSTED_MODEL_URL")
    if not url:
        return None
    token = os.environ.get("AEGIS_HOSTED_MODEL_TOKEN", "")
    model = os.environ.get("AEGIS_HOSTED_MODEL_NAME", "gpt-oss-20b")
    timeout = float(os.environ.get("AEGIS_HOSTED_MODEL_TIMEOUT_SECS", "6"))

    headers = {"Content-Type": "application/json"}
    if token:
        headers["Authorization"] = f"Bearer {token}"

    payload = {
        "model": model,
        "messages": [
            {"role": "system", "content": _SYSTEM_PROMPT},
            {"role": "user", "content": line},
        ],
        "max_tokens": 8,
        "temperature": 0,
    }

    try:
        with httpx.Client(timeout=timeout) as client:
            resp = client.post(url, headers=headers, json=payload)
            resp.raise_for_status()
            data = resp.json()
    except Exception as exc:
        log.debug("openai-compat hosted model call failed: %s", exc)
        return None

    return _normalise(_extract_openai_text(data))


def _extract_openai_text(data: dict) -> str:
    try:
        return str(data["choices"][0]["message"]["content"])
    except (KeyError, IndexError, TypeError):
        pass
    if isinstance(data.get("text"), str):
        return data["text"]
    try:
        return str(data["choices"][0]["text"])
    except (KeyError, IndexError, TypeError):
        return ""


def _normalise(text: str) -> ClassifyLabel | None:
    cleaned = text.strip().lower().strip(".,!\"'`")
    if "anomaly" in cleaned or "error" in cleaned:
        return "anomaly"
    if "routine" in cleaned or "normal" in cleaned or "info" in cleaned:
        return "routine"
    if "unknown" in cleaned:
        return "unknown"
    return None
