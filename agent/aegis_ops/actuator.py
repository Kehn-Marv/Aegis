"""Actuator: executes (or refuses) the decision against the right gateway.

The actuator never decides *what* to do — that's the reasoner's job.
It only decides whether the decision's exec mode permits it to call
the gateway, and translates the decision into the correct
`/api/command` payload.
"""

from __future__ import annotations

import logging

from .gateway_client import GatewayClient
from .models import Decision, ExecMode

log = logging.getLogger("aegis_ops.actuator")


class Actuator:
    def __init__(self, dry_run: bool = False):
        self.dry_run = dry_run

    async def act(
        self,
        decision: Decision,
        exec_mode: ExecMode,
        gateway: GatewayClient,
    ) -> tuple[str | None, str | None]:
        """Return `(result_message, error_message)`."""
        if exec_mode != "auto":
            return (None, None)
        if decision.action == "noop":
            return ("noop: gateway healthy", None)
        if self.dry_run:
            return (f"dry-run: would call {decision.action}", None)
        try:
            payload = await gateway.command(decision.action, decision.duration_secs)
            return (str(payload.get("message", payload)), None)
        except Exception as exc:
            log.warning("actuator failed: %s", exc)
            return (None, str(exc))
