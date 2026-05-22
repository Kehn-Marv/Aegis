"""Tests for hosted_model label normalisation."""

from __future__ import annotations

from aegis_sidecar.hosted_model import _normalise


def test_normalise_anomaly_variants() -> None:
    assert _normalise("Anomaly") == "anomaly"
    assert _normalise("this is an error") == "anomaly"


def test_normalise_routine_variants() -> None:
    assert _normalise("routine") == "routine"
    assert _normalise("normal traffic") == "routine"
    assert _normalise("INFO level") == "routine"


def test_normalise_unknown() -> None:
    assert _normalise("unknown") == "unknown"
    assert _normalise("maybe?") is None
