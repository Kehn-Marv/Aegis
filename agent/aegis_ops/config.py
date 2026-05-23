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

LLMTransportName = Literal["ollama", "splunk_ai"]


class GatewayCfg(BaseModel):
    name: str
    url: str


class AgentCfg(BaseModel):
    loop_interval_secs: int = 15
    dry_run: bool = False


class PolicyCfg(BaseModel):
    mode: PolicyMode = "low_risk_auto"
    min_confidence: float = 0.6
    cooldown_secs: int = 120


class SplunkCfg(BaseModel):
    """Splunk observability config. Empty `url` disables SPL queries."""

    url: str = ""
    token: str = ""
    verify_tls: bool = True
    earliest: str = "-5m"
    latest: str = "now"

    @property
    def enabled(self) -> bool:
        return bool(self.url and self.token)


class OllamaLLMCfg(BaseModel):
    url: str = "http://127.0.0.1:11434"
    # Default tuned for ~7 GB RAM machines. Qwen 2.5 is explicitly
    # designed for structured JSON output. Lower-spec alternatives:
    # "gemma2:2b" (~2 GB RAM) or "qwen2.5:1.5b" (~1.5 GB RAM).
    model: str = "qwen2.5:3b"
    timeout_secs: float = 60.0
    # When true, sends a JSON-schema `format` to Ollama so the model's
    # reply is forced to match the Decision schema. Big reliability win
    # on small models. Disable only if you're debugging prompt issues.
    enforce_schema: bool = True


class SplunkAILLMCfg(BaseModel):
    provider: str = "splunk_hosted"
    model: str = "gpt-oss-20b"
    timeout_secs: float = 30.0


class LLMCfg(BaseModel):
    transport: LLMTransportName = "ollama"
    ollama: OllamaLLMCfg = Field(default_factory=OllamaLLMCfg)
    splunk_ai: SplunkAILLMCfg = Field(default_factory=SplunkAILLMCfg)


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
