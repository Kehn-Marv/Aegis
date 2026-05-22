"""Pydantic models for everything that flows between agent stages.

The reasoner returns JSON that conforms to `Decision` — pydantic
validates it so a malformed model response can't crash the actuator.
"""

from __future__ import annotations

from typing import Literal

from pydantic import BaseModel, Field

Action = Literal["noop", "diagnostic", "override", "reset"]
PolicyMode = Literal["read_only", "low_risk_auto", "full_auto"]
ExecMode = Literal["auto", "recommend", "blocked"]


class GatewayStatus(BaseModel):
    """Snapshot returned by an Aegis gateway's `/api/status` endpoint."""

    uptime_secs: int
    online: bool
    override_active: bool
    diagnostic_active: bool
    queue_depth: int
    events_in: int
    events_out: int
    dedup_savings_pct: float
    unique_signatures: int


class TopSignature(BaseModel):
    signature: str
    count: int
    sample: str | None = None
    classification_label: str | None = None
    classification_confidence: float | None = None


TrajectoryLabel = Literal["stable", "degrading", "incident_likely"]


class Trends(BaseModel):
    """Rolling deltas the observer computes between successive ticks."""

    events_in_per_min: float = 0.0
    new_signatures_per_min: float = 0.0
    queue_depth_delta: int = 0
    anomaly_rate_per_min: float = 0.0
    signature_velocity_rising: bool = False
    queue_growing: bool = False
    trajectory: TrajectoryLabel = "stable"


class Observation(BaseModel):
    """Everything the reasoner needs to know about one gateway at one tick."""

    gateway: str
    gateway_url: str
    status: GatewayStatus
    top_signatures: list[TopSignature] = Field(default_factory=list)
    anomaly_count_5m: int = 0
    routine_count_5m: int = 0
    unknown_count_5m: int = 0
    trends: Trends = Field(default_factory=Trends)
    notes: list[str] = Field(default_factory=list)


class Decision(BaseModel):
    """The reasoner's verdict for one observation."""

    action: Action
    target_gateway: str
    duration_secs: int | None = None
    confidence: float = Field(ge=0.0, le=1.0)
    justification: str
    predicted_cost_impact_usd: float | None = None
    risk_factors: list[str] = Field(default_factory=list)


class DecisionRecord(BaseModel):
    """What gets shipped to Splunk under sourcetype=aegis:agent."""

    ts: float
    gateway: str
    observation: Observation
    decision: Decision
    exec_mode: ExecMode
    actuator_result: str | None = None
    actuator_error: str | None = None
    prompt: str | None = None
    raw_model_response: str | None = None
