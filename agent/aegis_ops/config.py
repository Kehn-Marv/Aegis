"""TOML configuration loader.

The agent supports three deployment shapes, picked by config alone:

* **Pure Ollama** — no Splunk credentials. The agent runs the autonomous
  loop end-to-end using a local Ollama LLM. SPL observations and HEC
  audit are skipped gracefully. Default Plan B for the hackathon.
* **Ollama + Splunk** — Splunk auth token for SPL observations + HEC
  token for audit. LLM reasoning still local. Works on any Splunk
  account including the locked-down trial.
* **Splunk-native** — `llm.transport = "splunk_ai"`. Reasoning runs in
  Splunk via `| ai`. Hibernated until SLIM API is provisioned (see
  `docs/splunk-blocker.md`).
"""

from __future__ import annotations

import tomllib
from pathlib import Path
from typing import Literal

from pydantic import BaseModel, Field, model_validator

from .models import PolicyMode

LLMTransportName = Literal["ollama", "splunk_ai", "aitk_ollama"]


class GatewayCfg(BaseModel):
    name: str
    url: str


class AgentCfg(BaseModel):
    loop_interval_secs: int = 15
    dry_run: bool = False


class ObserveCfg(BaseModel):
    """Optional observer add-ons.

    Today this only carries the CDTSM forecast settings, but it's a
    convenient place to grow any future observer-only switches without
    polluting `[agent]`.
    """

    cdtsm_enabled: bool = False
    cdtsm_horizon_minutes: int = 15
    cdtsm_history_window: str = "-2h"
    queue_forecast_spl: str = (
        "index=aegis sourcetype=aegis:selfmetric host={gateway} "
        "| timechart span=1m latest(queue_depth) AS queue_depth "
        "| apply CDTSM queue_depth time_field=_time forecast_k={horizon} "
        "  conf_interval=90 show_input=false"
    )
    queue_forecast_breach_threshold: int = 4096
    savings_forecast_spl: str = (
        "index=aegis sourcetype=aegis:selfmetric host={gateway} "
        "| timechart span=1m latest(dedup_savings_pct) AS dedup_savings_pct "
        "| apply CDTSM dedup_savings_pct time_field=_time forecast_k={horizon} "
        "  conf_interval=90 show_input=false"
    )
    savings_forecast_drop_threshold_pct: float = 75.0


class PolicyCfg(BaseModel):
    mode: PolicyMode = "low_risk_auto"
    min_confidence: float = 0.6
    cooldown_secs: int = 120


class SplunkMcpCfg(BaseModel):
    """Optional: route all observational SPL through the Splunk MCP Server.

    When `enabled` and `endpoint` are set, the agent becomes a real MCP
    client of `splunk_run_query` (Splunk Cloud Platform MCP Server v1.1)
    or `run_splunk_query` (Cisco-DevNet Splunk-MCP-Server-official).
    All search traffic appears in `index=_internal sourcetype=mcpjson`.
    Same auth token as the REST [splunk] block.

    Set `tool_name` to override auto-detection (e.g. for forks that
    rename the search tool).
    """

    enabled: bool = False
    endpoint: str = ""
    # Empty string is treated as "auto-detect" (the same as omitting the
    # field) so the TOML file can have a placeholder line.
    tool_name: str = ""
    verify_tls: bool = True
    timeout_secs: float = 30.0

    @property
    def tool_name_or_none(self) -> str | None:
        return self.tool_name.strip() or None


class SplunkCfg(BaseModel):
    """Splunk observability config. Empty `url` disables SPL queries."""

    url: str = ""
    token: str = ""
    verify_tls: bool = True
    earliest: str = "-5m"
    latest: str = "now"
    mcp: SplunkMcpCfg = Field(default_factory=SplunkMcpCfg)

    @property
    def enabled(self) -> bool:
        return bool(self.url and self.token)

    @property
    def mcp_enabled(self) -> bool:
        return self.mcp.enabled and bool(self.mcp.endpoint) and bool(self.token)


class OllamaLLMCfg(BaseModel):
    url: str = "http://127.0.0.1:11434"
    # Default matches the Splunk Hosted Models identifier `gpt-oss-20b`
    # (Ollama uses `gpt-oss:20b`). Needs ~13 GB on disk and ~16 GB RAM.
    # If you don't have the headroom, set this to a smaller model in
    # your `aegis-ops.toml` -- the example config documents the table.
    # Common fallbacks: "qwen2.5:7b" (~5 GB), "qwen2.5:3b" (~3 GB,
    # great JSON quality), "gemma2:2b" (~2 GB), "qwen2.5:1.5b" (~1.5 GB).
    model: str = "gpt-oss:20b"
    timeout_secs: float = 60.0
    # When true, sends a JSON-schema `format` to Ollama so the model's
    # reply is forced to match the Decision schema. Big reliability win
    # on small models. Disable only if you're debugging prompt issues.
    enforce_schema: bool = True


class SplunkAILLMCfg(BaseModel):
    """`| ai` SPL transport (AI Toolkit `ai` command).

    `provider` is the AITK Connection Management name (visible under
    Settings -> AI Toolkit -> Connections). Three real-world values:

    * `"splunk_hosted"` - the Splunk-Cloud SLIM-backed provider that
      exposes `gpt-oss-20b`, `gpt-oss-120b`, and `Foundation-Sec-1.1-8B`.
      Requires a Cloud account with SLIM provisioned (currently gated
      on the 14-day trial - see `docs/splunk-blocker.md`).
    * `"ollama_local"` - a user-defined Ollama LLM connection created in
      AITK Connection Management on **Splunk Enterprise (Developer
      License)**. No SLIM gating involved. See `docs/aitk-ollama.md`.
    * any other name - whatever the operator named their AITK LLM
      connection (Azure OpenAI, etc.).
    """

    provider: str = "splunk_hosted"
    model: str = "gpt-oss-20b"
    timeout_secs: float = 30.0


class AitkOllamaLLMCfg(BaseModel):
    """Sugar for `splunk_ai` pre-wired to an AITK Ollama connection.

    Equivalent to `transport = "splunk_ai"` with
    `provider = "ollama_local"` and `model = "gpt-oss:20b"`, but reads
    better in operator-facing config files because it makes the routing
    explicit: this is the Ollama-via-Splunk-AITK path, not the raw
    Ollama direct path.
    """

    provider: str = "ollama_local"
    model: str = "gpt-oss:20b"
    timeout_secs: float = 30.0


class LLMCfg(BaseModel):
    transport: LLMTransportName = "ollama"
    ollama: OllamaLLMCfg = Field(default_factory=OllamaLLMCfg)
    splunk_ai: SplunkAILLMCfg = Field(default_factory=SplunkAILLMCfg)
    aitk_ollama: AitkOllamaLLMCfg = Field(default_factory=AitkOllamaLLMCfg)


class AuditCfg(BaseModel):
    """HEC audit shipping. Empty endpoint disables audit (dry log only)."""

    hec_endpoint: str = ""
    hec_token: str = ""
    hec_index: str = "aegis"
    hec_source: str = "aegis:agent"
    hec_sourcetype: str = "aegis:agent"
    verify_tls: bool = False

    @property
    def enabled(self) -> bool:
        return bool(self.hec_endpoint and self.hec_token)


class AegisOpsCfg(BaseModel):
    agent: AgentCfg = Field(default_factory=AgentCfg)
    observe: ObserveCfg = Field(default_factory=ObserveCfg)
    policy: PolicyCfg = Field(default_factory=PolicyCfg)
    llm: LLMCfg = Field(default_factory=LLMCfg)
    splunk: SplunkCfg = Field(default_factory=SplunkCfg)
    audit: AuditCfg = Field(default_factory=AuditCfg)
    gateways: list[GatewayCfg] = Field(min_length=1)

    @model_validator(mode="before")
    @classmethod
    def _back_compat(cls, data: dict) -> dict:
        """Accept legacy `[hosted_model]` block as `[llm.splunk_ai]`."""
        if isinstance(data, dict) and "hosted_model" in data and "llm" not in data:
            data = {**data, "llm": {"transport": "splunk_ai", "splunk_ai": data["hosted_model"]}}
        return data

    @classmethod
    def load(cls, path: str | Path) -> "AegisOpsCfg":
        text = Path(path).read_text(encoding="utf-8")
        data = tomllib.loads(text)
        return cls.model_validate(data)
