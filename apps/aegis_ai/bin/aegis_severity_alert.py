# Copyright (c) 2026 Aegis Contributors
# Licensed under the MIT License (see LICENSE at repo root).
#
# Aegis Severity Assessment - Custom Alert Action.
#
# Triggered by the [Aegis Severity Assessment] saved search whenever the
# gateway emits a collapsed `aegis:metric` event with an abnormally high
# suppressed-line count. The handler:
#
#   1. Reads the saved search's CSV results (gzipped temp file).
#   2. Builds a structured AlertData object.
#   3. Invokes a splunklib.ai.Agent backed by an OpenAI-compatible LLM
#      (default: local Ollama running gpt-oss:20b) with a Pydantic
#      output schema, guaranteeing a structured verdict back.
#   4. Indexes the verdict to `sourcetype=aegis_ai:assessment` so it
#      flows into the Aegis dashboards alongside the rest of the
#      observability data.
#
# Modelled on the splunk-sdk-python example
# (examples/ai_custom_alert_app/bin/threat_level_assessment.py) so the
# pattern is immediately recognisable to Splunk reviewers.

import asyncio
import csv
import gzip
import json
import os
import sys
from collections.abc import Sequence
from typing import Literal
from urllib.parse import urlsplit

# Required only for splunk-sdk-python CI/CD; harmless on real Splunk.
sys.path.insert(0, "/splunklib-deps")
# Include any 3rd-party deps shipped in /bin/lib/.
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "lib"))

from pydantic import BaseModel, Field

from setup_logging import setup_logging  # noqa: E402
from llm_factory import build_llm_model  # noqa: E402

from splunklib import client  # noqa: E402
from splunklib.ai.agent import Agent  # noqa: E402

# Some Splunk packages override SSL_CERT_FILE to a path that doesn't
# exist on every distro. Mirror the SDK example's defensive unset.
CA_TRUST_STORE = "/opt/splunk/openssl/cert.pem"
if os.environ.get("SSL_CERT_FILE") == CA_TRUST_STORE and not os.path.exists(CA_TRUST_STORE):
    del os.environ["SSL_CERT_FILE"]

APP_NAME = "aegis_ai"
logger = setup_logging(APP_NAME)

SYSTEM_PROMPT = """You are an Aegis observability incident analyst. Your role
is to look at suppressed-signature collapse events emitted by the Aegis edge
gateway and decide what an on-call engineer should do next.

You will receive:
- a `search_name` identifying the saved search that fired the alert, and
- `search_results`, a list of one or more rows. Each row corresponds to a
  signature the gateway collapsed during a single dedup window. Relevant
  fields per row:
    * signature              - blake3 hash of the structural log signature
    * count                  - how many raw lines were collapsed
    * window_secs            - width of the dedup window in seconds
    * sample                 - a redacted sample of the original log line
    * service                - service name extracted from the line (since v0.2)
    * classification.label   - anomaly | routine | unknown
    * classification.confidence - 0..1
    * classification.strategy   - splunk_ai | openai_compat | embedding_distance | keyword

You MUST respond with a single JSON object matching the schema you've been
given. No commentary, no markdown, no surrounding text. Be conservative:
recommend `override` only when there is strong reason to suspect an active
incident that the on-call needs raw logs for. Prefer `diagnostic` or `noop`
when in doubt; Aegis's own decision card (sourcetype=aegis:decision) is the
authoritative source for what to do next.
"""


class AlertResultRow(BaseModel):
    """Loose row shape - real keys vary depending on the saved search."""

    model_config = {"extra": "allow"}


class AlertData(BaseModel):
    search_name: str
    search_results: Sequence[dict[str, str]]


class SeverityAssessment(BaseModel):
    """Structured verdict the LLM must produce."""

    severity: Literal["low", "medium", "high", "critical"] = Field(
        description="How urgent this collapse pattern is."
    )
    confidence: float = Field(
        ge=0.0, le=1.0, description="LLM confidence in its severity call (0..1)."
    )
    summary: str = Field(
        description="2-3 sentence plain-English summary of what's happening."
    )
    recommended_aegis_action: Literal["noop", "status", "diagnostic", "override"] = Field(
        description="Which Aegis MCP tool the operator should consider calling next.",
    )
    recommended_duration_secs: int = Field(
        default=0,
        ge=0,
        le=600,
        description=(
            "Seconds to keep diagnostic/override active. 0 for noop/status. "
            "Cap is 600 (10 minutes) to keep cost-control invariants intact."
        ),
    )
    rationale: str = Field(
        description="Brief justification for the recommended_aegis_action.",
    )


async def invoke_agent(service: client.Service, alert_data: AlertData) -> SeverityAssessment:
    model = build_llm_model()
    logger.info(f"Invoking model={model.model} base_url={model.base_url}")

    async with Agent(
        model=model,
        system_prompt=SYSTEM_PROMPT,
        service=service,
        output_schema=SeverityAssessment,
    ) as agent:
        result = await agent.invoke_with_data(
            instructions=(
                "Assess these collapsed Aegis signatures and decide whether the on-call "
                "should switch the gateway into a higher-fidelity mode. Stay conservative."
            ),
            data=alert_data.model_dump(),
        )
        return result.structured_output


def _read_results(path: str) -> list[dict[str, str]]:
    rows: list[dict[str, str]] = []
    with gzip.open(path, "rt", encoding="utf-8", errors="replace") as fh:
        reader = csv.DictReader(fh)
        for row in reader:
            rows.append(dict(row))
    logger.debug(f"loaded {len(rows)} rows from {path}")
    return rows


def handle_alert() -> None:
    payload_json = sys.stdin.read()
    try:
        payload = json.loads(payload_json)
    except Exception as exc:
        logger.exception(f"could not parse alert payload: {exc}")
        sys.exit(1)

    results_file_path = payload.get("results_file", "")
    if not results_file_path:
        logger.error("no results_file in alert payload")
        sys.exit(1)

    try:
        search_results = _read_results(results_file_path)
        if not search_results:
            logger.info("no rows in saved search results; nothing to assess")
            return

        search_name = payload.get("search_name", "")
        alert_data = AlertData(search_name=search_name, search_results=search_results)

        server_uri = payload.get("server_uri", "https://localhost:8089")
        splunk_uri = urlsplit(server_uri, scheme="https")
        session_key = payload.get("session_key", "")
        service = client.connect(
            scheme=splunk_uri.scheme,
            token=session_key,
            host=splunk_uri.hostname,
            port=splunk_uri.port,
            autologin=True,
        )

        assessment = asyncio.run(invoke_agent(service, alert_data))
        logger.info(
            f"assessment severity={assessment.severity} "
            f"confidence={assessment.confidence:.2f} "
            f"recommended={assessment.recommended_aegis_action} "
            f"duration={assessment.recommended_duration_secs}s"
        )

        configuration = payload.get("configuration", {})
        output_index = configuration.get("output_index", "aegis")
        output_sourcetype = configuration.get("output_sourcetype", "assessment")

        envelope = {
            "search_name": search_name,
            "row_count": len(search_results),
            "assessment": assessment.model_dump(),
        }

        idx: client.Index = service.indexes[output_index]
        idx.submit(
            json.dumps(envelope),
            sourcetype=f"{APP_NAME}:{output_sourcetype}",
        )
        logger.info(f"indexed assessment to index={output_index} sourcetype={APP_NAME}:{output_sourcetype}")
    except Exception as exc:
        logger.exception(f"alert handler failed: {exc}")
        sys.exit(2)


if __name__ == "__main__":
    handle_alert()
