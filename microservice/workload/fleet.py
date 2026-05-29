"""The simulated service fleet and the incident scenarios.

Log line shapes deliberately match what the Aegis gateway already parses
(`LEVEL [ts] service: message`) so the causal-chain and dedup engines treat
this app exactly like a real one. Timings are tight enough to fit inside the
default 60s causal window.
"""

from __future__ import annotations

import logging
from dataclasses import dataclass, field


@dataclass(frozen=True)
class ServiceSpec:
    name: str
    role: str
    depends_on: tuple[str, ...] = ()
    base_latency_ms: float = 40.0
    base_error_rate: float = 0.004
    weight: int = 10  # relative share of routine traffic


# A small but realistic e-commerce call graph. The edge is api-gateway; money
# flows through payment-api; orders sit at the end of the chain.
FLEET: list[ServiceSpec] = [
    ServiceSpec("api-gateway", "Edge router / TLS termination", ("auth", "checkout"), 18, 0.002, 20),
    ServiceSpec("auth", "Authentication and session issuance", ("session-cache",), 24, 0.003, 16),
    ServiceSpec("session-cache", "Hot path cache for active user sessions", (), 6, 0.001, 14),
    ServiceSpec("checkout", "Customer-facing checkout flow", ("payment-api", "inventory"), 70, 0.005, 14),
    ServiceSpec("payment-api", "Handles all transaction processing", ("ledger-db",), 95, 0.006, 12),
    ServiceSpec("orders", "Order fulfilment pipeline", ("checkout",), 55, 0.004, 10),
    ServiceSpec("inventory", "Stock levels and reservations", (), 30, 0.003, 8),
    ServiceSpec("ledger-db", "Double-entry transaction ledger", (), 22, 0.002, 6),
    ServiceSpec("notifications", "Email / push receipts", (), 40, 0.004, 6),
]

FLEET_BY_NAME = {s.name: s for s in FLEET}

# Routine, healthy traffic. {ph} = placeholder filled in by the simulator.
ROUTINE_LOGS: dict[str, list[str]] = {
    "api-gateway": [
        "INFO  [{ts}] api-gateway: 200 GET /v1/catalog rid={rid} latency={lat}ms",
        "INFO  [{ts}] api-gateway: 200 POST /v1/cart rid={rid} latency={lat}ms",
    ],
    "auth": [
        "INFO  [{ts}] auth: issued session token user={uid} latency={lat}ms",
        "DEBUG [{ts}] auth: token refresh user={uid}",
    ],
    "session-cache": [
        "DEBUG [{ts}] session-cache: hit key=session:{uid}",
    ],
    "checkout": [
        "INFO  [{ts}] checkout: 200 POST /v1/checkout rid={rid} latency={lat}ms",
    ],
    "payment-api": [
        "INFO  [{ts}] payment-api: charge ok amount={amt} rid={rid} latency={lat}ms",
    ],
    "orders": [
        "INFO  [{ts}] orders: order created id={oid} rid={rid} latency={lat}ms",
    ],
    "inventory": [
        "INFO  [{ts}] inventory: reserved sku={sku} qty={qty} latency={lat}ms",
    ],
    "ledger-db": [
        "DEBUG [{ts}] ledger-db: committed txn={txn} latency={lat}ms",
    ],
    "notifications": [
        "INFO  [{ts}] notifications: receipt sent to user={uid}",
    ],
}


@dataclass(frozen=True)
class Step:
    at: float           # seconds after scenario start
    service: str
    level: int
    template: str
    error: bool = False


@dataclass(frozen=True)
class Scenario:
    key: str
    title: str
    summary: str
    severity: str       # "red" | "orange"
    duration_secs: float
    steps: tuple[Step, ...]
    # Services whose latency/error behaviour degrades for the scenario window.
    degrade: dict[str, dict] = field(default_factory=dict)


INFO = logging.INFO
WARN = logging.WARNING
ERROR = logging.ERROR


SCENARIOS: list[Scenario] = [
    Scenario(
        key="cascade",
        title="Payment cascade",
        summary="payment-api DB pool exhausts; checkout then orders fail behind it.",
        severity="red",
        duration_secs=26,
        degrade={
            "payment-api": {"error_rate": 0.85, "latency_mult": 6.0},
            "checkout": {"error_rate": 0.6, "latency_mult": 3.0},
            "orders": {"error_rate": 0.5, "latency_mult": 2.0},
        },
        steps=(
            Step(0.0, "payment-api", ERROR, "ERROR [{ts}] payment-api: db connection pool exhausted (rid={rid})", True),
            Step(0.6, "payment-api", ERROR, "ERROR [{ts}] payment-api: caused by tokio::sync::AcquireError (rid={rid})", True),
            Step(1.2, "payment-api", ERROR, "ERROR [{ts}] payment-api: charge request {rid} timed out after 30s", True),
            Step(4.0, "checkout", ERROR, "ERROR [{ts}] checkout: payment-api unreachable (rid={rid})", True),
            Step(4.4, "checkout", WARN, "WARN  [{ts}] checkout: marking session={uid} as abandoned"),
            Step(8.0, "orders", ERROR, "ERROR [{ts}] orders: cannot create order, checkout never completed (rid={rid})", True),
            Step(8.3, "orders", ERROR, "ERROR [{ts}] orders: at orders::create (orders.rs:54) precondition missing (rid={rid})", True),
        ),
    ),
    Scenario(
        key="crashloop",
        title="Auth crash-loop",
        summary="auth crash-loops the same connection-refused stack thousands of times.",
        severity="red",
        duration_secs=18,
        degrade={"auth": {"error_rate": 0.9, "latency_mult": 1.5}},
        steps=tuple(
            Step(i * 0.4, "auth", ERROR, tmpl, True)
            for i, tmpl in enumerate(
                [
                    "ERROR [{ts}] auth: connection refused to session-cache 10.0.4.{oct}:6379 (rid={rid})",
                    "ERROR [{ts}] auth: at cache::Pool::checkout (cache.rs:142)",
                    "ERROR [{ts}] auth: at handlers::login (handlers.rs:88)",
                    "ERROR [{ts}] auth: caused by io::Error ConnectionRefused",
                ]
                * 3
            )
        ),
    ),
    Scenario(
        key="latency",
        title="Checkout latency spike",
        summary="session-cache slows down; auth and checkout latency climbs into the red.",
        severity="orange",
        duration_secs=22,
        degrade={
            "session-cache": {"error_rate": 0.05, "latency_mult": 14.0},
            "auth": {"error_rate": 0.1, "latency_mult": 5.0},
            "checkout": {"error_rate": 0.08, "latency_mult": 3.0},
        },
        steps=(
            Step(0.0, "session-cache", WARN, "WARN  [{ts}] session-cache: eviction storm, p99 latency={lat}ms"),
            Step(3.0, "auth", WARN, "WARN  [{ts}] auth: session lookup slow latency={lat}ms (rid={rid})"),
            Step(6.0, "checkout", WARN, "WARN  [{ts}] checkout: upstream auth slow, queue depth rising (rid={rid})"),
        ),
    ),
    Scenario(
        key="silence",
        title="Silent notifications",
        summary="notifications stops emitting entirely — absence is the signal.",
        severity="orange",
        duration_secs=70,
        degrade={"notifications": {"silent": True}},
        steps=(
            Step(0.0, "notifications", WARN, "WARN  [{ts}] notifications: worker shutting down for redeploy"),
        ),
    ),
]

SCENARIOS_BY_KEY = {s.key: s for s in SCENARIOS}
