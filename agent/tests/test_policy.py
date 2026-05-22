"""Tests for the PolicyEngine — the safety rail that decides whether a
Decision auto-actuates or is downgraded to a recommendation."""

from __future__ import annotations

import time

from aegis_ops.models import Decision
from aegis_ops.policy import PolicyEngine


def _decision(action: str, confidence: float = 0.9, target: str = "us-east") -> Decision:
    return Decision(
        action=action,  # type: ignore[arg-type]
        target_gateway=target,
        duration_secs=30 if action in {"diagnostic", "override"} else None,
        confidence=confidence,
        justification="test",
    )


def test_low_risk_auto_executes_diagnostic_and_noop():
    p = PolicyEngine(mode="low_risk_auto", cooldown_secs=0)
    assert p.classify(_decision("diagnostic")) == "auto"
    assert p.classify(_decision("noop")) == "auto"


def test_low_risk_auto_recommends_override_and_reset():
    p = PolicyEngine(mode="low_risk_auto", cooldown_secs=0)
    assert p.classify(_decision("override")) == "recommend"
    assert p.classify(_decision("reset")) == "recommend"


def test_read_only_recommends_everything_except_noop_pathways():
    p = PolicyEngine(mode="read_only", cooldown_secs=0)
    assert p.classify(_decision("override")) == "recommend"
    assert p.classify(_decision("diagnostic")) == "recommend"
    assert p.classify(_decision("reset")) == "recommend"


def test_full_auto_executes_everything():
    p = PolicyEngine(mode="full_auto", cooldown_secs=0)
    assert p.classify(_decision("override")) == "auto"
    assert p.classify(_decision("reset")) == "auto"
    assert p.classify(_decision("diagnostic")) == "auto"


def test_min_confidence_downgrades_to_recommend():
    p = PolicyEngine(mode="full_auto", min_confidence=0.8, cooldown_secs=0)
    assert p.classify(_decision("override", confidence=0.5)) == "recommend"
    assert p.classify(_decision("override", confidence=0.9)) == "auto"


def test_cooldown_blocks_repeat_within_window():
    p = PolicyEngine(mode="full_auto", cooldown_secs=60)
    first = p.classify(_decision("override"))
    assert first == "auto"
    # Same gateway + same action within the cooldown -> blocked.
    second = p.classify(_decision("override"))
    assert second == "blocked"


def test_cooldown_does_not_block_different_actions():
    p = PolicyEngine(mode="full_auto", cooldown_secs=60)
    assert p.classify(_decision("override")) == "auto"
    assert p.classify(_decision("diagnostic")) == "auto"


def test_cooldown_does_not_block_different_gateways():
    p = PolicyEngine(mode="full_auto", cooldown_secs=60)
    assert p.classify(_decision("override", target="us-east")) == "auto"
    assert p.classify(_decision("override", target="eu-west")) == "auto"


def test_noop_is_exempt_from_cooldown_and_min_confidence():
    p = PolicyEngine(mode="full_auto", min_confidence=0.99, cooldown_secs=999)
    assert p.classify(_decision("noop", confidence=0.0)) == "auto"
    assert p.classify(_decision("noop", confidence=0.0)) == "auto"


def test_cooldown_expires():
    p = PolicyEngine(mode="full_auto", cooldown_secs=0)
    assert p.classify(_decision("override")) == "auto"
    # Simulate time passing past the cooldown.
    p._last_fire.clear()
    assert p.classify(_decision("override")) == "auto"


def test_low_risk_auto_blocks_repeat_diagnostic_within_cooldown():
    p = PolicyEngine(mode="low_risk_auto", cooldown_secs=60)
    assert p.classify(_decision("diagnostic")) == "auto"
    assert p.classify(_decision("diagnostic")) == "blocked"
    # But noop is still allowed
    assert p.classify(_decision("noop")) == "auto"


# Cosmetic smoke: trends/observation models import cleanly.
def test_models_importable():
    from aegis_ops.models import DecisionRecord, GatewayStatus, Observation, Trends  # noqa: F401
    assert True
