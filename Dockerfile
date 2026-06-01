#
#
# Single-container build for the whole Aegis project:
#   - Rust gateway  (aegis-daemon)        → ingest 5140, REST/MCP/UI on 7321
#   - React control panel (built, served by the gateway)
#   - Python workload microservice        → web UI + telemetry on 8080
#
# Build:  docker build -t aegis .
# Run:    docker run --rm -p 7321:7321 -p 8080:8080 aegis
#   Aegis control panel → http://localhost:7321
#   Workload control room → http://localhost:8080
#
# Base images are pinned for reproducibility. Dependency versions are locked
# by Cargo.lock (Rust), package-lock.json (UI), and requirements.txt (Python).

# ---------------------------------------------------------------------------
# Stage 1 — build the Rust gateway (release)
# ---------------------------------------------------------------------------
FROM rust:1-bookworm AS rust-builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY gateway ./gateway
RUN cargo build --release --bin aegis-daemon

# ---------------------------------------------------------------------------
# Stage 2 — build the control-panel UI
# ---------------------------------------------------------------------------
FROM node:22-bookworm-slim AS ui-builder
WORKDIR /ui
COPY ui/package.json ui/package-lock.json ./
RUN npm ci
COPY ui/ ./
RUN npm run build

# ---------------------------------------------------------------------------
# Stage 3 — runtime: one container, all services
# ---------------------------------------------------------------------------
FROM python:3.12-slim-bookworm AS runtime

ENV PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1

RUN apt-get update \
 && apt-get install -y --no-install-recommends supervisor curl ca-certificates \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Python dependencies (pinned) for the workload microservice
COPY microservice/requirements.txt /app/microservice/requirements.txt
RUN pip install --no-cache-dir -r /app/microservice/requirements.txt

# Application sources + built artifacts
COPY microservice/ /app/microservice/
COPY configs/ /app/configs/
COPY --from=rust-builder /build/target/release/aegis-daemon /usr/local/bin/aegis-daemon
COPY --from=ui-builder /ui/dist /app/ui
COPY docker/supervisord.conf /etc/supervisor/conf.d/aegis.conf

# Non-root user + writable data directory for the SQLite stores
RUN useradd --create-home --uid 10001 appuser \
 && mkdir -p /app/data \
 && chown -R appuser:appuser /app
USER appuser

ENV AEGIS_UI_DIR=/app/ui \
    WORKLOAD_PORT=8080 \
    AEGIS_GATEWAY_HOST=127.0.0.1 \
    AEGIS_GATEWAY_PORT=5140 \
    SIM_AUTOPILOT=true \
    PYTHONPATH=/app/microservice

# 7321 control panel + REST + MCP · 8080 workload UI · 5140 raw log ingest
EXPOSE 7321 8080 5140 5141/udp

HEALTHCHECK --interval=15s --timeout=4s --start-period=25s --retries=4 \
  CMD curl -fsS http://127.0.0.1:7321/api/health && curl -fsS http://127.0.0.1:8080/api/health || exit 1

CMD ["supervisord", "-c", "/etc/supervisor/conf.d/aegis.conf"]
