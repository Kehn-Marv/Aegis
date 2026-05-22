"""Tests for reasoner decision parsing."""

from __future__ import annotations

import json

import pytest
from pydantic import ValidationError

from aegis_ops.models import GatewayStatus, Observation
from aegis_ops.reasoner import Reasoner


def _observation() -> Observation:
    return Observation(
        gateway="us-east",
        gateway_url="http://127.0.0.1:7321",
        status=GatewayStatus(
            uptime_secs=120,
            online=True,
            override_active=False,
            diagnostic_active=False,
            queue_depth=0,
            events_in=1000,
            events_out=50,
            dedup_savings_pct=99.2,
            unique_signatures=12,
        ),
    )


def test_extract_text_ai_response() -> None:
    rows = [{"ai_response": '{"action":"noop"}'}]
    assert Reasoner._extract_text(rows).startswith("{")


def test_parse_decision_from_fenced_json() -> None:
    raw = """Here is my answer:
```json
{"action":"diagnostic","duration_secs":60,"confidence":0.8,"justification":"test","risk_factors":[]}
```"""
    decision = Reasoner._parse_decision(raw, _observation())
    assert decision.action == "diagnostic"
    assert decision.target_gateway == "us-east"
    assert decision.duration_secs == 60


def test_parse_decision_sets_gateway_default() -> None:
    raw = json.dumps(
        {
            "action": "noop",
            "confidence": 0.95,
            "justification": "healthy",
            "risk_factors": [],
        }
    )
    decision = Reasoner._parse_decision(raw, _observation())
    assert decision.target_gateway == "us-east"


def test_safe_noop_on_invalid_json() -> None:
    with pytest.raises((ValidationError, ValueError, json.JSONDecodeError)):
        Reasoner._parse_decision("not json at all", _observation())
