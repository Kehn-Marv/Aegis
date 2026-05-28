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

    model_config = {"extra": "ignore"}

    uptime_secs: int
    online: bool
    override_active: bool
    diagnostic_active: bool
    queue_depth: int
    events_in: int
    events_out: int
    dedup_savings_pct: float
    unique_signatures: int
    # Fields added in Aegis v0.2 (four-pillar rewrite). Tolerated as
    # optional so old gateways still work with new agents.
    state: str = "green"
    incidents_remembered: int = 0


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


class CdtsmForecast(BaseModel):
    """Summary of a Splunk-Hosted CDTSM forecast for one metric.

    Populated by the observer when `[observe] cdtsm_enabled = true` and a
    Splunk client is available. Fed into the reasoner prompt so the LLM
    can act on a prediction, not just on the current state.
    """

    metric: str
    horizon_minutes: int
    peak_predicted: float
    minutes_to_peak: int
    threshold: float
    breached: bool
    confidence_band_pct: int = 90


class CausalChainSummary(BaseModel):
    """Compact summary of the most recent causal chain the gateway detected."""

    chain_id: str
    root_cause_service: str
    confidence: float
    services: list[str] = Field(default_factory=list)
    headline: str | None = None


class IncidentMatchSummary(BaseModel):
    """One similar past incident the gateway surfaced in its decision card."""

    incident_id: str
    similarity: float
    past_root_cause_service: str
    past_cause: str | None = None
    past_fix: str | None = None
    past_resolved_in_minutes: int | None = None


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
    forecasts: list[CdtsmForecast] = Field(default_factory=list)
    notes: list[str] = Field(default_factory=list)
    # New in v0.2: surface the gateway's own decision card + memory matches
    # so the LLM has them in context without re-deriving from SPL.
    causal_chain: CausalChainSummary | None = None
    similar_past_incidents: list[IncidentMatchSummary] = Field(default_factory=list)


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
