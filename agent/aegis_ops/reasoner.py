"""Reasoner: call a Splunk Hosted Model via SPL `| ai` and parse a Decision.

The contract:
    POST /services/search/jobs/oneshot  with SPL of the form
        | makeresults
        | eval prompt="<system + user prompt>"
        | ai prompt=prompt provider=<provider> model=<model>

The `ai` command writes the model's reply into a column (typically
named `ai_response`). We extract that text and pydantic-validate it
as a `Decision`. Anything malformed becomes a safe `noop` decision so
the agent loop never crashes.
"""

from __future__ import annotations

import json
import logging
import re

from pydantic import ValidationError

from .models import Decision, Observation
from .prompts import build_full_prompt
from .splunk_client import SplunkClient

log = logging.getLogger("aegis_ops.reasoner")


class Reasoner:
    def __init__(
        self,
        splunk: SplunkClient,
        provider: str = "splunk_hosted",
        model: str = "gpt-oss-20b",
    ):
        self.splunk = splunk
        self.provider = provider
        self.model = model

    async def reason(self, obs: Observation) -> tuple[Decision, str, str]:
        """Return `(decision, prompt, raw_model_response)`.

        Tuple is what `auditor.record` needs to ship to Splunk.
        """
        prompt = build_full_prompt(obs)
        spl = self._build_spl(prompt)
        try:
            rows = await self.splunk.oneshot(spl)
        except Exception as exc:
            log.warning("hosted model SPL failed: %s", exc)
            return self._safe_noop(obs, f"reasoner_error: {exc}"), prompt, ""

        raw = self._extract_text(rows)
        try:
            decision = self._parse_decision(raw, obs)
            return decision, prompt, raw
        except (ValidationError, ValueError, json.JSONDecodeError) as exc:
            log.warning("hosted model returned unparseable decision: %s; raw=%r", exc, raw[:200])
            return self._safe_noop(obs, f"parse_error: {exc}"), prompt, raw

    # ------------------------------------------------------------------
    # internals
    # ------------------------------------------------------------------

    def _build_spl(self, prompt_text: str) -> str:
        # SPL string escaping: double quotes inside the prompt must be
        # backslash-escaped, and SPL itself uses double quotes for
        # string literals.
        escaped = prompt_text.replace("\\", "\\\\").replace('"', '\\"')
        return (
            f'| makeresults '
            f'| eval prompt="{escaped}" '
            f'| ai prompt=prompt provider={self.provider} model={self.model}'
        )

    @staticmethod
    def _extract_text(rows: list[dict]) -> str:
        """The `ai` command's column name varies by AITK version; try the
        common ones in order."""
        if not rows:
            return ""
        row = rows[0]
        for key in ("ai_response", "response", "answer", "text", "ai"):
            v = row.get(key)
            if isinstance(v, str) and v.strip():
                return v
        # As a last resort, return the whole row joined together.
        return " ".join(str(v) for v in row.values() if isinstance(v, str))

    @staticmethod
    def _parse_decision(raw: str, obs: Observation) -> Decision:
        # Models sometimes wrap JSON in ``` blocks; strip them.
        candidate = raw.strip()
        candidate = re.sub(r"^```(?:json)?\s*", "", candidate)
        candidate = re.sub(r"\s*```$", "", candidate)
        # If there's prose around the JSON, grab the largest {...} block.
        m = re.search(r"\{.*\}", candidate, re.DOTALL)
        if m:
            candidate = m.group(0)
        data = json.loads(candidate)
        # Normalise target_gateway to the observed one (model sometimes guesses).
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
