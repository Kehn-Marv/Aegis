"""Reasoner: call an LLM transport with a prompt, parse a Decision.

The transport is pluggable (`transports.LLMTransport`). Today we ship
two implementations:

* `OllamaTransport` (default) — local LLM, no Splunk credentials needed.
* `SplunkAITransport` — Splunk Hosted Models via SPL `| ai`. Hibernated
  while the 14-day Cloud trial blocks SLIM API access
  (see `docs/splunk-blocker.md`).

The reasoner itself doesn't care which transport is active — same
prompt, same `Decision` schema, same safe-noop fallback when the model
returns anything malformed.
"""

from __future__ import annotations

import json
import logging
import re

from pydantic import ValidationError

from .models import Decision, Observation
from .prompts import build_full_prompt
from .transports import LLMTransport

log = logging.getLogger("aegis_ops.reasoner")


class Reasoner:
    def __init__(self, transport: LLMTransport):
        self.transport = transport

    async def reason(self, obs: Observation) -> tuple[Decision, str, str]:
        """Return `(decision, prompt, raw_model_response)`."""
        prompt = build_full_prompt(obs)
        try:
            raw = await self.transport.call(prompt)
        except Exception as exc:
            log.warning("transport %s failed: %s", self.transport.name, exc)
            return self._safe_noop(obs, f"transport_error: {exc}"), prompt, ""

        if not raw:
            return self._safe_noop(obs, "transport_returned_empty"), prompt, raw

        try:
            decision = self._parse_decision(raw, obs)
            return decision, prompt, raw
        except (ValidationError, ValueError, json.JSONDecodeError) as exc:
            log.warning(
                "transport %s returned unparseable decision: %s; raw=%r",
                self.transport.name,
                exc,
                raw[:200],
            )
            return self._safe_noop(obs, f"parse_error: {exc}"), prompt, raw

    @staticmethod
    def _parse_decision(raw: str, obs: Observation) -> Decision:
        candidate = raw.strip()
        candidate = re.sub(r"^```(?:json)?\s*", "", candidate)
        candidate = re.sub(r"\s*```$", "", candidate)
        m = re.search(r"\{.*\}", candidate, re.DOTALL)
        if m:
            candidate = m.group(0)
        data = json.loads(candidate)
        data.setdefault("target_gateway", obs.gateway)
        return Decision.model_validate(data)

    @staticmethod
    def _safe_noop(obs: Observation, reason: str) -> Decision:
        return Decision(
            action="noop",
            target_gateway=obs.gateway,
            duration_secs=None,
            confidence=0.0,
            justification=f"agent defaulted to noop: {reason}",
            predicted_cost_impact_usd=0.0,
            risk_factors=["reasoner_unavailable"],
        )
