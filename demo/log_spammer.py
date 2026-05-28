"""Generate log traffic to demo Aegis dedup, causal chains, and incident memory.

Patterns:
    crashloop  — same stack trace many times → demo dedup-to-metric
    routine    — INFO/200 OK traffic         → demo routine summarisation
    mixed      — realistic blend             → demo classifier behaviour
    cascade    — payment-api breaks first, checkout follows 8s later,
                 orders 16s later → demo causal chain + incident memory
    silence    — start one service, then go quiet → demo silent detector

Usage examples:
    python demo/log_spammer.py --target tcp://127.0.0.1:5140 \
        --pattern cascade --duration 60

    python demo/log_spammer.py --target tcp://127.0.0.1:5140 \
        --pattern crashloop --rate 200 --duration 10
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
    "ERROR [{ts}] payment-api: connection refused to 10.0.4.{ip}:5432 (rid={rid})",
    "  at db::Pool::checkout (db.rs:142)",
    "  at handlers::charge (handlers.rs:88)",
    "  at runtime::task::poll (runtime.rs:303)",
    "  caused by: io::Error: ConnectionRefused",
]

ROUTINE_TEMPLATES = [
    "INFO  [{ts}] api: 200 OK GET /v1/users/{id} latency={lat}ms",
    "INFO  [{ts}] api: 200 OK POST /v1/orders rid={rid} latency={lat}ms",
    "INFO  [{ts}] api: 200 OK GET /v1/orders/{id} latency={lat}ms",
    "DEBUG [{ts}] cache: hit key=session:{id}",
]

ANOMALY_TEMPLATES = [
    "WARN  [{ts}] auth: rate-limit exceeded for ip=10.{ip1}.{ip2}.{ip3}",
    "ERROR [{ts}] storage: write failed bucket=audit object={uuid} retry=3",
]

# Cascade pattern: one realistic multi-service outage. payment-api dies first
# (its DB pool is exhausted). ~4s later checkout starts timing out on
# payments. ~8s later orders fails because checkout never completes. Aegis
# attributes the root cause to payment-api (earliest first-fire) and stores
# the chain as a fingerprint in incident memory.
#
# Timings are deliberately tight so the demo fits inside short causal
# windows (10–30s). Production deployments using the default 60s causal
# window will catch much longer cascades just fine.
CASCADE_SCRIPT = [
    # (delay_secs, service, line_template)
    (0.0, "payment-api", "ERROR [{ts}] payment-api: db connection pool exhausted (rid={rid})"),
    (0.5, "payment-api", "  caused by: tokio::sync::AcquireError"),
    (1.0, "payment-api", "ERROR [{ts}] payment-api: charge request {rid} timed out after 30s"),
    (4.0, "checkout", "ERROR [{ts}] checkout: payment-api unreachable (rid={rid})"),
    (4.3, "checkout", "WARN  [{ts}] checkout: marking session={id} as abandoned"),
    (8.0, "orders", "ERROR [{ts}] orders: cannot create order, checkout never completed (rid={rid})"),
    (8.3, "orders", "  at orders::create (orders.rs:54)"),
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
    "routine":   emit_routine,
    "mixed":     emit_mixed,
}


def run_cascade(send) -> None:
    """One pass of the cascade script. Reproducible 16-second mini-outage.

    Each line is rendered with fresh timestamps + request ids so the dedup
    engine collapses repeats inside a window but the causal-chain detector
    sees each service's first-fire timestamp clearly.
    """
    start = time.monotonic()
    for delay, _service, template in CASCADE_SCRIPT:
        wait = (start + delay) - time.monotonic()
        if wait > 0:
            time.sleep(wait)
        # Send the templated line a handful of times so dedup has something
        # interesting to collapse for each signature.
        for _ in range(5):
            send(render(template))


def run_silence(send) -> None:
    """Emit one burst, then stop. Pairs with a daemon configured to flag
    a service as silent after ~30s of quiet.
    """
    for _ in range(5):
        send(render("INFO  [{ts}] heartbeat-svc: ok"))
        time.sleep(0.1)
    print("silence pattern: emitted; now sleeping for 120s so the silent-service detector fires",
          file=sys.stderr)
    time.sleep(120)


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
    ap.add_argument(
        "--pattern",
        choices=list(PATTERNS) + ["cascade", "silence"],
        default="mixed",
    )
    ap.add_argument("--rate", type=int, default=100, help="events per second (rate-based patterns)")
    ap.add_argument("--duration", type=int, default=0, help="seconds (0 = forever for rate-based)")
    args = ap.parse_args()

    send, sock = make_sender(args.target)
    try:
        if args.pattern == "cascade":
            print("cascade pattern: ~16 seconds, three services break in sequence "
                  "(payment-api → checkout → orders). Watch for the [CHAIN ...] "
                  "and [DECIDE state=red ...] events in the daemon log.",
                  file=sys.stderr)
            run_cascade(send)
            print("cascade complete.", file=sys.stderr)
            return
        if args.pattern == "silence":
            run_silence(send)
            return

        emit = PATTERNS[args.pattern]
        interval = 1.0 / max(args.rate, 1)
        deadline = time.monotonic() + args.duration if args.duration > 0 else None
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
