"""Tests for Splunk `| ai` SPL helpers."""

from __future__ import annotations

from aegis_sidecar.splunk_ai import build_ai_spl, extract_ai_text


def test_build_ai_spl_escapes_quotes() -> None:
    spl = build_ai_spl('say "hello"', provider="splunk_hosted", model="gpt-oss-20b")
    assert '\\"hello\\"' in spl
    assert "provider=splunk_hosted" in spl
    assert "model=gpt-oss-20b" in spl


def test_extract_ai_text_prefers_ai_response() -> None:
    rows = [{"ai_response": "routine", "other": "ignored"}]
    assert extract_ai_text(rows) == "routine"


def test_extract_ai_text_fallback_column() -> None:
    rows = [{"response": "anomaly"}]
    assert extract_ai_text(rows) == "anomaly"


def test_extract_ai_text_empty() -> None:
    assert extract_ai_text([]) == ""
