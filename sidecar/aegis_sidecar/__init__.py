"""Aegis AI sidecar package."""

from .classifier import Classification, Classifier
from .embeddings import Embedder

__all__ = ["Classification", "Classifier", "Embedder", "server"]
