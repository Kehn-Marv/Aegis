"""KMeans clustering for log embeddings.

Used to group log lines that vary too much in surface form for the
gateway's structural-hash dedup to catch (e.g. templated INFO messages
where bare numbers + UUIDs were already masked but the templated phrase
still differs).
"""

from __future__ import annotations

import math
from typing import Sequence

import numpy as np
from sklearn.cluster import KMeans


def auto_k(n_samples: int, *, cap: int = 16) -> int:
    """Pick a sensible `k` based on dataset size."""
    if n_samples <= 2:
        return 1
    k = int(round(math.sqrt(n_samples / 2)))
    return max(1, min(cap, k))


def cluster(embeddings: Sequence[Sequence[float]], k: int | None = None) -> tuple[list[int], int]:
    """Cluster `embeddings` and return `(labels, n_clusters)`.

    If `k` is None, an `auto_k` heuristic picks the count.
    """
    arr = np.asarray(embeddings, dtype=np.float32)
    if arr.ndim != 2 or arr.shape[0] == 0:
        return ([], 0)
    n = arr.shape[0]
    chosen_k = k if k is not None else auto_k(n)
    chosen_k = max(1, min(chosen_k, n))
    if chosen_k == 1:
        return ([0] * n, 1)
    model = KMeans(n_clusters=chosen_k, n_init="auto", random_state=42)
    labels = model.fit_predict(arr)
    return (labels.tolist(), chosen_k)
