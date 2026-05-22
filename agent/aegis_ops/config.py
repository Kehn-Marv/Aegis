"""TOML configuration loader."""

from __future__ import annotations

import tomllib
from pathlib import Path

from pydantic import BaseModel, Field

from .models import PolicyMode


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
    url: str
    token: str
    verify_tls: bool = True
    earliest: str = "-5m"
    latest: str = "now"


class HostedModelCfg(BaseModel):
    provider: str = "splunk_hosted"
    model: str = "gpt-oss-20b"
    timeout_secs: int = 30


class AuditCfg(BaseModel):
    hec_endpoint: str
    hec_token: str
    hec_index: str = "aegis"
    hec_source: str = "aegis:agent"
    hec_sourcetype: str = "aegis:agent"
    verify_tls: bool = False


class AegisOpsCfg(BaseModel):
    agent: AgentCfg = Field(default_factory=AgentCfg)
    policy: PolicyCfg = Field(default_factory=PolicyCfg)
    splunk: SplunkCfg
    hosted_model: HostedModelCfg = Field(default_factory=HostedModelCfg)
    audit: AuditCfg
    gateways: list[GatewayCfg] = Field(min_length=1)

    @classmethod
    def load(cls, path: str | Path) -> "AegisOpsCfg":
        text = Path(path).read_text(encoding="utf-8")
        data = tomllib.loads(text)
        return cls.model_validate(data)
