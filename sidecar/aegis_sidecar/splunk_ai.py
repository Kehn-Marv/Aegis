"""Splunk AI Toolkit adapter — invoke Hosted Models via SPL `| ai`.

This is the **authentic** Splunk integration path for the sidecar:
classification runs inside Splunk's search pipeline using the same
`| ai` command the AegisOps agent uses for reasoning.

Environment variables (preferred over OpenAI-compatible URLs):

    AEGIS_SPLUNK_URL          Splunk base URL, e.g. https://prd-p-XXXX.splunkcloud.com
    AEGIS_SPLUNK_TOKEN        Splunk auth token (needs `search` + `apply_ai_commander_command`)
    AEGIS_SPLUNK_AI_PROVIDER  AITK provider label. Default: splunk_hosted
    AEGIS_SPLUNK_AI_MODEL     Model id. Default: gpt-oss-20b
    AEGIS_SPLUNK_VERIFY_TLS   true/false. Default: true
    AEGIS_SPLUNK_TIMEOUT_SECS Per-request timeout. Default: 12

If URL or token is unset, `is_configured()` is false and the classifier
falls back to local embeddings.
"""

from __future__ import annotations

import logging
import os

import httpx

log = logging.getLogger("aegis.splunk_ai")


def is_configured() -> bool:
    return bool(os.environ.get("AEGIS_SPLUNK_URL") and os.environ.get("AEGIS_SPLUNK_TOKEN"))


def classify(line: str, *, system_prompt: str) -> str | None:
    """Run a one-shot SPL search with `| ai` and return raw model text."""
    url = os.environ.get("AEGIS_SPLUNK_URL", "").rstrip("/")
    token = os.environ.get("AEGIS_SPLUNK_TOKEN", "")
    if not url or not token:
        return None

    provider = os.environ.get("AEGIS_SPLUNK_AI_PROVIDER", "splunk_hosted")
    model = os.environ.get("AEGIS_SPLUNK_AI_MODEL", "gpt-oss-20b")
    verify = os.environ.get("AEGIS_SPLUNK_VERIFY_TLS", "true").lower() not in {
        "0",
        "false",
        "no",
    }
    timeout = float(os.environ.get("AEGIS_SPLUNK_TIMEOUT_SECS", "12"))

    prompt = f"{system_prompt}\n\nLog line:\n{line}"
    spl = build_ai_spl(prompt, provider=provider, model=model)

    data = {
        "search": spl,
        "output_mode": "json",
        "earliest_time": "0",
        "latest_time": "now",
        "exec_mode": "oneshot",
    }

    try:
        with httpx.Client(verify=verify, timeout=timeout) as client:
            resp = client.post(
                f"{url}/services/search/jobs/oneshot",
                data=data,
                headers={"Authorization": f"Bearer {token}"},
            )
            resp.raise_for_status()
            payload = resp.json()
    except Exception as exc:
        log.debug("splunk | ai classify failed: %s", exc)
        return None

    rows = payload.get("results", [])
    return extract_ai_text(rows)


def build_ai_spl(prompt_text: str, *, provider: str, model: str) -> str:
    """Build SPL for `| makeresults | eval prompt=... | ai ...`."""
    escaped = prompt_text.replace("\\", "\\\\").replace('"', '\\"')
    return (
        f'| makeresults '
        f'| eval prompt="{escaped}" '
        f'| ai prompt=prompt provider={provider} model={model}'
    )


def extract_ai_text(rows: list[dict]) -> str:
    """Extract model output from an `| ai` result row."""
    if not rows:
        return ""
    row = rows[0]
    for key in ("ai_response", "response", "answer", "text", "ai"):
        value = row.get(key)
        if isinstance(value, str) and value.strip():
            return value
    return " ".join(str(v) for v in row.values() if isinstance(v, str))
