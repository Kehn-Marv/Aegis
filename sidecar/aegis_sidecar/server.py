"""Aegis AI sidecar: FastAPI service for embeddings, clustering, and classification.

This service is intentionally narrow and fast: it never owns long-lived
state, never persists, and falls back gracefully when models or hosted
endpoints are unavailable. The Rust gateway calls it for higher-resolution
log analysis than the local structural-hash dedup can provide.
"""

from __future__ import annotations

import logging
import os
import time
from typing import Literal

import uvicorn
from fastapi import FastAPI
from pydantic import BaseModel, Field

from . import clustering, hosted_model
from .classifier import Classifier
from .embeddings import Embedder

logging.basicConfig(level=os.environ.get("AEGIS_LOG_LEVEL", "INFO"))
log = logging.getLogger("aegis.sidecar")

app = FastAPI(title="Aegis Sidecar", version="0.2.0")

_classifier: Classifier | None = None


def get_classifier() -> Classifier:
    global _classifier
    if _classifier is None:
        _classifier = Classifier()
    return _classifier


class EmbedRequest(BaseModel):
    lines: list[str] = Field(..., min_length=1)


class EmbedResponse(BaseModel):
    dim: int
    embeddings: list[list[float]]
    model: str
    fallback: bool


class ClusterRequest(BaseModel):
    embeddings: list[list[float]] = Field(..., min_length=1)
    k: int | None = Field(default=None, ge=1)


class ClusterResponse(BaseModel):
    labels: list[int]
    n_clusters: int


class ClusterLinesRequest(BaseModel):
    lines: list[str] = Field(..., min_length=1)
    k: int | None = Field(default=None, ge=1)


class ClusterLinesResponse(BaseModel):
    labels: list[int]
    n_clusters: int
    model: str
    fallback: bool


class ClassifyRequest(BaseModel):
    line: str


class ClassifyResponse(BaseModel):
    label: Literal["anomaly", "routine", "unknown"]
    confidence: float
    strategy: str
    latency_ms: float


class InfoResponse(BaseModel):
    embedding_model: str
    embedding_dim: int
    embedding_fallback: bool
    hosted_model_configured: bool
    hosted_model_transport: str | None
    hosted_model_name: str | None


@app.get("/health")
def health() -> dict[str, str]:
    return {"status": "ok"}


@app.get("/info", response_model=InfoResponse)
def info() -> InfoResponse:
    embedder = Embedder.instance()
    transport = hosted_model.transport()
    model_name = None
    if transport == "splunk_ai":
        model_name = os.environ.get("AEGIS_SPLUNK_AI_MODEL", "gpt-oss-20b")
    elif transport == "openai_compat":
        model_name = os.environ.get("AEGIS_HOSTED_MODEL_NAME", "gpt-oss-20b")
    return InfoResponse(
        embedding_model=embedder.model_name,
        embedding_dim=embedder.dim,
        embedding_fallback=embedder.using_fallback,
        hosted_model_configured=hosted_model.is_configured(),
        hosted_model_transport=transport,
        hosted_model_name=model_name,
    )


@app.post("/embed", response_model=EmbedResponse)
def embed(req: EmbedRequest) -> EmbedResponse:
    embedder = Embedder.instance()
    vecs = embedder.encode(req.lines)
    return EmbedResponse(
        dim=embedder.dim,
        embeddings=vecs.tolist(),
        model=embedder.model_name,
        fallback=embedder.using_fallback,
    )


@app.post("/cluster", response_model=ClusterResponse)
def cluster(req: ClusterRequest) -> ClusterResponse:
    labels, n = clustering.cluster(req.embeddings, k=req.k)
    return ClusterResponse(labels=labels, n_clusters=n)


@app.post("/cluster_lines", response_model=ClusterLinesResponse)
def cluster_lines(req: ClusterLinesRequest) -> ClusterLinesResponse:
    embedder = Embedder.instance()
    vecs = embedder.encode(req.lines)
    labels, n = clustering.cluster(vecs.tolist(), k=req.k)
    return ClusterLinesResponse(
        labels=labels,
        n_clusters=n,
        model=embedder.model_name,
        fallback=embedder.using_fallback,
    )


@app.post("/classify", response_model=ClassifyResponse)
def classify(req: ClassifyRequest) -> ClassifyResponse:
    t0 = time.perf_counter()
    result = get_classifier().classify(req.line)
    elapsed_ms = (time.perf_counter() - t0) * 1000.0
    return ClassifyResponse(
        label=result.label,
        confidence=result.confidence,
        strategy=result.strategy,
        latency_ms=elapsed_ms,
    )


def main() -> None:
    port = int(os.environ.get("AEGIS_SIDECAR_PORT", "8765"))
    host = os.environ.get("AEGIS_SIDECAR_HOST", "127.0.0.1")
    uvicorn.run("aegis_sidecar.server:app", host=host, port=port, reload=False)


if __name__ == "__main__":
    main()
