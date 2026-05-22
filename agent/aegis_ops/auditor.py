"""Auditor: ships every decision (prompt + raw response + action + result)
to Splunk under sourcetype=aegis:agent.

This is the accountability story. An SRE skeptical of the agent should
be able to open Splunk and read every reasoning step the agent has
ever made, including the prompt and the model's raw response.
"""

from __future__ import annotations

import logging
import socket
import time

from .config import AuditCfg
from .models import DecisionRecord
from .splunk_client import HecClient

log = logging.getLogger("aegis_ops.auditor")


class Auditor:
    def __init__(self, cfg: AuditCfg, dry_run: bool = False):
        self.cfg = cfg
        self.dry_run = dry_run
        self.host = socket.gethostname()
        self._hec: HecClient | None = None
        if not dry_run:
            self._hec = HecClient(
                endpoint=cfg.hec_endpoint,
                token=cfg.hec_token,
                verify_tls=cfg.verify_tls,
            )

    async def close(self) -> None:
        if self._hec is not None:
            await self._hec.close()

    async def record(self, record: DecisionRecord) -> None:
        if self.dry_run or self._hec is None:
            log.info(
                "DRY-RUN audit gw=%s action=%s exec=%s conf=%.2f",
                record.gateway,
                record.decision.action,
                record.exec_mode,
                record.decision.confidence,
            )
            return
        event = {
            "time": time.time(),
            "host": self.host,
            "source": self.cfg.hec_source,
            "sourcetype": self.cfg.hec_sourcetype,
            "index": self.cfg.hec_index,
            "event": record.model_dump(mode="json"),
        }
        try:
            await self._hec.send(event)
        except Exception as exc:
            log.warning("audit emit failed: %s", exc)
