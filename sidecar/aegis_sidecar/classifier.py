"""Hybrid log classifier.

Strategy order, each falls back to the next on failure:

1. **Splunk Hosted Model** — if `AEGIS_SPLUNK_URL` + `AEGIS_SPLUNK_TOKEN`
   are set, classify via SPL `| ai` (preferred). Otherwise, if
   `AEGIS_HOSTED_MODEL_URL` is set, use an OpenAI-compatible endpoint.
   Highest fidelity, slowest path.
2. **Embedding-distance** — cosine similarity against precomputed
   anomaly/routine centroids built from seed phrases at construction time.
   This is the path the gateway uses by default — fast, local, and
   private. It runs entirely inside the Splunk security boundary.
3. **Keyword heuristic** — final fallback if embeddings are unavailable.
   Cheap, last-resort signal so the API never returns `unknown` purely
   because nothing answered.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass
from typing import Literal

import numpy as np

from .embeddings import Embedder
from . import hosted_model

log = logging.getLogger("aegis.classifier")

Label = Literal["anomaly", "routine", "unknown"]

ANOMALY_SEEDS = [
    "ERROR connection refused to database",
    "FATAL out of memory exception",
    "panic: nil pointer dereference",
    "Exception: timeout while reading from socket",
    "HTTP 500 internal server error",
    "stack trace traceback at unwind",
    "service unavailable retry exceeded",
    "broker rejected message after 3 retries",
]

ROUTINE_SEEDS = [
    "INFO 200 OK GET /api/v1/users",
    "DEBUG cache hit for session key",
    "INFO user logged in successfully",
    "200 OK POST /v1/orders request completed",
    "INFO request handled latency 42ms",
    "DEBUG heartbeat received from peer",
    "INFO scheduled job finished without errors",
]

ANOMALY_KEYWORDS = ("error", "fatal", "panic", "exception", "traceback", "5xx", "refused", "timeout")
ROUTINE_KEYWORDS = ("info", "debug", "200 ok", "request completed", "heartbeat")


@dataclass
class Classification:
    label: Label
    confidence: float
    strategy: str


class Classifier:
    """Singleton-style classifier; cheap to keep alive globally."""

    def __init__(self) -> None:
        self._embedder = Embedder.instance()
        self._anomaly_centroid: np.ndarray | None = None
        self._routine_centroid: np.ndarray | None = None
        self._build_centroids()

    def _build_centroids(self) -> None:
        try:
            anomaly = self._embedder.encode(ANOMALY_SEEDS)
            routine = self._embedder.encode(ROUTINE_SEEDS)
            self._anomaly_centroid = _normalise(anomaly.mean(axis=0))
            self._routine_centroid = _normalise(routine.mean(axis=0))
        except Exception as exc:
            log.warning("failed to build classifier centroids: %s", exc)

    def classify(self, line: str) -> Classification:
        if hosted_model.is_configured():
            hosted, strategy = hosted_model.classify(line)
            if hosted is not None and strategy is not None:
                return Classification(label=hosted, confidence=0.95, strategy=strategy)

        if self._anomaly_centroid is not None and self._routine_centroid is not None:
            vec = _normalise(self._embedder.encode([line])[0])
            anomaly_sim = float(np.dot(vec, self._anomaly_centroid))
            routine_sim = float(np.dot(vec, self._routine_centroid))
            label, confidence = _score_to_label(anomaly_sim, routine_sim)
            return Classification(label=label, confidence=confidence, strategy="embedding_distance")

        return _keyword(line)


def _score_to_label(anomaly_sim: float, routine_sim: float) -> tuple[Label, float]:
    if abs(anomaly_sim - routine_sim) < 0.05:
        return ("unknown", 0.5)
    if anomaly_sim > routine_sim:
        return ("anomaly", _clip(anomaly_sim))
    return ("routine", _clip(routine_sim))


def _keyword(line: str) -> Classification:
    lower = line.lower()
    if any(w in lower for w in ANOMALY_KEYWORDS):
        return Classification(label="anomaly", confidence=0.7, strategy="keyword")
    if any(w in lower for w in ROUTINE_KEYWORDS):
        return Classification(label="routine", confidence=0.6, strategy="keyword")
    return Classification(label="unknown", confidence=0.4, strategy="keyword")


def _normalise(vec: np.ndarray) -> np.ndarray:
    norm = float(np.linalg.norm(vec))
    if norm == 0.0:
        return vec
    return vec / norm


def _clip(score: float) -> float:
    return max(0.0, min(1.0, (score + 1.0) / 2.0))
