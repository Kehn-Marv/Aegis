"""Entrypoint: `python -m workload`.

Starts the web server; the simulator and telemetry stack come up with it
via the FastAPI lifespan, so this is the only command you ever run.
"""

from __future__ import annotations

import uvicorn

from .config import settings


def main() -> None:
    uvicorn.run(
        "workload.server:app",
        host=settings.host,
        port=settings.port,
        log_level="info",
        access_log=False,
    )


if __name__ == "__main__":
    main()
