"""Tests for the Aegis classifier fallback chain.

These tests intentionally do *not* require the `sentence-transformers`
model to be downloadable — they exercise the embedding-distance path
through the hash-based fallback embedder, plus the keyword fallback.
"""

from __future__ import annotations

import os

import pytest

# Ensure hosted-model paths are disabled for these tests so we exercise the
# local embedding and keyword paths deterministically.
os.environ.pop("AEGIS_HOSTED_MODEL_URL", None)
os.environ.pop("AEGIS_SPLUNK_URL", None)
os.environ.pop("AEGIS_SPLUNK_TOKEN", None)

from aegis_sidecar.classifier import Classifier, _keyword  # noqa: E402
from aegis_sidecar.embeddings import Embedder  # noqa: E402


@pytest.fixture(scope="module")
def classifier() -> Classifier:
    return Classifier()


def test_keyword_fallback_anomaly() -> None:
    r = _keyword("ERROR connection refused to 10.0.0.1:5432")
    assert r.label == "anomaly"
    assert r.strategy == "keyword"


def test_keyword_fallback_routine() -> None:
    r = _keyword("INFO 200 OK GET /v1/users/42 latency=33ms")
    assert r.label == "routine"
    assert r.strategy == "keyword"


def test_keyword_fallback_unknown() -> None:
    r = _keyword("the moon is made of green cheese")
    assert r.label == "unknown"


def test_classify_returns_valid_label(classifier: Classifier) -> None:
    r = classifier.classify("ERROR connection refused while charging customer")
    assert r.label in {"anomaly", "routine", "unknown"}
    assert 0.0 <= r.confidence <= 1.0
    assert r.strategy in {"splunk_ai", "openai_compat", "embedding_distance", "keyword"}


def test_classify_is_deterministic_on_fallback_embedder() -> None:
    # The fallback hash-embedder is deterministic, so two calls with the
    # same input must produce the same classification.
    embedder = Embedder.instance()
    if not embedder.using_fallback:
        pytest.skip("real embedder loaded; determinism is approximate")
    c = Classifier()
    a = c.classify("FATAL out of memory")
    b = c.classify("FATAL out of memory")
    assert a.label == b.label
    assert a.strategy == b.strategy
