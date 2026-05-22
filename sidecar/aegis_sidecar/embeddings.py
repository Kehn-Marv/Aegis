"""Sentence embeddings.

Loads `sentence-transformers/all-MiniLM-L6-v2` (~80MB) lazily on first call.
The model name is overridable via the `AEGIS_EMBEDDING_MODEL` env var so we
can swap in a heavier or domain-tuned model without code changes.

If `sentence-transformers` isn't importable in the environment (offline
machine, missing wheels), we fall back to a deterministic 32-dimensional
hash-based embedding so the rest of the pipeline keeps working.
"""

from __future__ import annotations

import hashlib
import os
import threading
from typing import Sequence

import numpy as np

DEFAULT_MODEL = "sentence-transformers/all-MiniLM-L6-v2"
_FALLBACK_DIM = 32


class Embedder:
    """Thread-safe singleton wrapper around a sentence-transformer."""

    _instance: "Embedder | None" = None
    _lock = threading.Lock()

    @classmethod
    def instance(cls) -> "Embedder":
        with cls._lock:
            if cls._instance is None:
                cls._instance = cls()
            return cls._instance

    def __init__(self) -> None:
        self.model_name = os.environ.get("AEGIS_EMBEDDING_MODEL", DEFAULT_MODEL)
        self._model = None
        self._dim: int | None = None
        self._using_fallback = False
        self._load()

    def _load(self) -> None:
        try:
            from sentence_transformers import SentenceTransformer  # type: ignore
        except ImportError:
            self._using_fallback = True
            self._dim = _FALLBACK_DIM
            return
        try:
            self._model = SentenceTransformer(self.model_name)
            self._dim = int(self._model.get_sentence_embedding_dimension())
        except Exception:
            # Network down or model missing locally — fall back gracefully.
            self._using_fallback = True
            self._dim = _FALLBACK_DIM
            self._model = None

    @property
    def dim(self) -> int:
        assert self._dim is not None
        return self._dim

    @property
    def using_fallback(self) -> bool:
        return self._using_fallback

    def encode(self, lines: Sequence[str]) -> np.ndarray:
        if self._using_fallback or self._model is None:
            return _hash_embed(lines, dim=self.dim)
        return np.asarray(
            self._model.encode(list(lines), normalize_embeddings=True),
            dtype=np.float32,
        )


def _hash_embed(lines: Sequence[str], dim: int = _FALLBACK_DIM) -> np.ndarray:
    """Deterministic fallback embedding. Useful for tests + offline dev."""
    out = np.zeros((len(lines), dim), dtype=np.float32)
    for i, line in enumerate(lines):
        digest = hashlib.blake2b(line.encode("utf-8"), digest_size=dim).digest()
        out[i] = np.frombuffer(digest, dtype=np.uint8).astype(np.float32) / 255.0
        norm = float(np.linalg.norm(out[i]))
        if norm > 0:
            out[i] /= norm
    return out
