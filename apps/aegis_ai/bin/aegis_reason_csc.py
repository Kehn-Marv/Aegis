# Copyright (c) 2026 Aegis Contributors
# Licensed under the MIT License (see LICENSE at repo root).
#
# Aegis Reason - Custom Search Command (`| aegisreason`).
#
# Usage in SPL:
#   index="aegis" sourcetype="aegis:agent" | head 10
#       | aegisreason context="fleet_operations"
#
# Pipes each input event through a splunklib.ai.Agent and adds two new
# fields to every record:
#   * aegis_ai_recommendation       - JSON of the structured recommendation
#   * aegis_ai_recommendation_text  - flat string for easy table display
#
# Built on the splunk-sdk-python EventingCommand pattern so it is
# immediately recognisable to Splunkbase reviewers and to anyone who
# has read the official AI custom search app example.

import asyncio
import os
import sys
from collections.abc import Generator, Sequence
from typing import Any, Literal, final, override

# Required only for splunk-sdk-python CI/CD; harmless on real Splunk.
sys.path.insert(0, "/splunklib-deps")
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "lib"))

from pydantic import BaseModel, Field

from setup_logging import setup_logging  # noqa: E402
from llm_factory import build_llm_model  # noqa: E402

from splunklib.ai.agent import Agent  # noqa: E402
from splunklib.data import Record  # noqa: E402
from splunklib.searchcommands import (  # noqa: E402
    Configuration,
    Option,
    dispatch,  # pyright: ignore[reportPrivateLocalImportUsage]
    validators,
)
from splunklib.searchcommands.eventing_command import EventingCommand  # noqa: E402

CA_TRUST_STORE = "/opt/splunk/openssl/cert.pem"
if os.environ.get("SSL_CERT_FILE") == CA_TRUST_STORE and not os.path.exists(CA_TRUST_STORE):
    del os.environ["SSL_CERT_FILE"]

APP_NAME = "aegis_ai"
logger = setup_logging(APP_NAME)


class Recommendation(BaseModel):
    next_action: Literal["noop", "status", "diagnostic", "override", "reset"] = Field(
        description="The Aegis MCP tool to call next."
    )
    duration_secs: int = Field(
        default=0,
        ge=0,
        le=600,
        description="Seconds the action should persist (0 for noop/status/reset).",
    )
    confidence: float = Field(ge=0.0, le=1.0)
    rationale: str = Field(description="One-sentence justification.")


SYSTEM_PROMPT_TEMPLATE = """You are an Aegis fleet-operations co-pilot.

You are reviewing a single Aegis agent decision record (or any other event
the operator has piped into you). Given the record contents and the operator's
high-level context ({context}), recommend exactly ONE next Aegis MCP action.

Available actions:
  * noop       - no action needed; the situation is normal
  * status     - re-read live gateway status (read-only)
  * diagnostic - enable verbose tracing for a bounded window (low-risk)
  * override   - disable dedup compression and stream raw to HEC (HIGH-RISK)
  * reset      - clear queue + in-memory dedup table (HIGH-RISK)

Respond with a single JSON object matching the schema you've been given.
Stay conservative: prefer `noop` or `status` over destructive actions.
"""


@final
@Configuration()
class AegisReasonCSC(EventingCommand):
    """Add an LLM-recommended next Aegis action to each input record."""

    context = Option(
        doc="Operator context string to include in the LLM prompt (e.g. fleet_operations, incident_drill).",
        require=False,
        default="fleet_operations",
        validate=validators.Match("context", r"^[A-Za-z0-9_\-]{1,64}$"),
    )

    @override
    def transform(self, records: Sequence[Record]) -> Generator[Record, Any, Any]:
        logger.info(f"aegisreason begin: context={self.context}")
        for record in records:
            if not record:
                continue
            try:
                rec = asyncio.run(self._reason_about(record))
                record["aegis_ai_recommendation"] = rec.model_dump_json()
                record["aegis_ai_recommendation_text"] = (
                    f"{rec.next_action}"
                    + (f"({rec.duration_secs}s)" if rec.duration_secs else "")
                    + f" conf={rec.confidence:.2f}: {rec.rationale}"
                )
            except Exception as exc:
                logger.exception(f"aegisreason error on one record: {exc}")
                record["aegis_ai_recommendation_error"] = str(exc)
            yield record
        logger.info("aegisreason end")

    async def _reason_about(self, record: Record) -> Recommendation:
        if not self.service:
            raise AssertionError("no Splunk service handle available to the command")

        model = build_llm_model()
        async with Agent(
            model=model,
            system_prompt=SYSTEM_PROMPT_TEMPLATE.format(context=self.context),
            service=self.service,
            output_schema=Recommendation,
        ) as agent:
            result = await agent.invoke_with_data(
                instructions="Recommend the next Aegis MCP action for this record.",
                data=dict(record),
            )
            return result.structured_output


dispatch(AegisReasonCSC, sys.argv, sys.stdin, sys.stdout, __name__)
