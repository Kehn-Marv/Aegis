"""Generate log traffic to demo Aegis dedup and summarization.

Patterns:
    crashloop  — emit the same stack trace many times to demo dedup-to-metric
    routine    — emit standard INFO/2xx logs to demo summarization
    mixed      — a realistic mix of both, with occasional anomalies

Usage:
    python demo/log_spammer.py --target tcp://127.0.0.1:5140 --pattern crashloop --rate 200
"""

from __future__ import annotations

import argparse
import random
import socket
import sys
import time
import uuid
from urllib.parse import urlparse

CRASHLOOP_STACK = [
    "ERROR [{ts}] payment-service: connection refused to 10.0.4.{ip}:5432 (rid={rid})",
    "  at db::Pool::checkout (db.rs:142)",
    "  at handlers::charge (handlers.rs:88)",
    "  at runtime::task::poll (runtime.rs:303)",
    "  caused by: io::Error: ConnectionRefused",
]

ROUTINE_TEMPLATES = [
    'INFO  [{ts}] api: 200 OK GET /v1/users/{id} latency={lat}ms',
    'INFO  [{ts}] api: 200 OK POST /v1/orders rid={rid} latency={lat}ms',
    'INFO  [{ts}] api: 200 OK GET /v1/orders/{id} latency={lat}ms',
    'DEBUG [{ts}] cache: hit key=session:{id}',
]

ANOMALY_TEMPLATES = [
    'WARN  [{ts}] auth: rate-limit exceeded for ip=10.{ip1}.{ip2}.{ip3}',
    'ERROR [{ts}] storage: write failed bucket=audit object={uuid} retry=3',
]


def now_ts() -> str:
    return time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())


def render(template: str) -> str:
    return template.format(
        ts=now_ts(),
        ip=random.randint(2, 254),
        ip1=random.randint(2, 254),
        ip2=random.randint(2, 254),
        ip3=random.randint(2, 254),
        id=random.randint(1, 10000),
        rid=uuid.uuid4().hex[:12],
        uuid=str(uuid.uuid4()),
        lat=random.randint(8, 120),
    )


def emit_crashloop(send) -> None:
    for line in CRASHLOOP_STACK:
        send(render(line))


def emit_routine(send) -> None:
    send(render(random.choice(ROUTINE_TEMPLATES)))


def emit_mixed(send) -> None:
    r = random.random()
    if r < 0.05:
        emit_crashloop(send)
    elif r < 0.10:
        send(render(random.choice(ANOMALY_TEMPLATES)))
    else:
        emit_routine(send)


PATTERNS = {
    "crashloop": emit_crashloop,
    "routine": emit_routine,
    "mixed": emit_mixed,
}


def make_sender(target: str):
    url = urlparse(target)
    if url.scheme == "tcp":
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect((url.hostname, url.port))

        def send(line: str) -> None:
            sock.sendall((line + "\n").encode("utf-8"))

        return send, sock
    if url.scheme == "udp":
        sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        addr = (url.hostname, url.port)

        def send(line: str) -> None:
            sock.sendto((line + "\n").encode("utf-8"), addr)

        return send, sock
    if url.scheme == "stdout":
        def send(line: str) -> None:
            sys.stdout.write(line + "\n")
            sys.stdout.flush()

        return send, None
    raise SystemExit(f"unsupported target scheme: {url.scheme!r}")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument(
        "--target",
        default="stdout://",
        help="tcp://host:port | udp://host:port | stdout:// (default)",
    )
    ap.add_argument("--pattern", choices=list(PATTERNS), default="mixed")
    ap.add_argument("--rate", type=int, default=100, help="events per second")
    ap.add_argument("--duration", type=int, default=0, help="seconds (0 = forever)")
    args = ap.parse_args()

    emit = PATTERNS[args.pattern]
    send, sock = make_sender(args.target)
    interval = 1.0 / max(args.rate, 1)
    deadline = time.monotonic() + args.duration if args.duration > 0 else None
    try:
        while deadline is None or time.monotonic() < deadline:
            emit(send)
            time.sleep(interval)
    except KeyboardInterrupt:
        pass
    finally:
        if sock is not None:
            sock.close()


if __name__ == "__main__":
    main()
