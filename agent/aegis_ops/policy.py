"""Policy: decides whether a `Decision` should auto-actuate or be downgraded
to a recommendation.

Three modes:

* **read_only** — every decision becomes a recommendation. The agent
  observes and reports; an operator decides.
* **low_risk_auto** (default) — `diagnostic` and `noop` auto-execute;
  `override` and `reset` become recommendations.
* **full_auto** — every decision auto-executes (use carefully).

A per-tool cooldown also prevents the agent from firing the same tool
twice within `cooldown_secs` for the same gateway, so a single
prompt-response loop can't accidentally call `override` ten times in a row.
"""

from __future__ import annotations

import time
from dataclasses import dataclass, field

from .models import Action, Decision, ExecMode, PolicyMode


LOW_RISK_ACTIONS: set[Action] = {"noop", "diagnostic"}
ALL_ACTIONS: set[Action] = {"noop", "diagnostic", "override", "reset"}


@dataclass
class PolicyEngine:
    mode: PolicyMode = "low_risk_auto"
    min_confidence: float = 0.6
    cooldown_secs: int = 120
    _last_fire: dict[tuple[str, str], float] = field(default_factory=dict)

    def classify(self, decision: Decision) -> ExecMode:
        """Return what the actuator should do with this decision."""
        now = time.time()
        cooldown_key = (decision.target_gateway, decision.action)
        last = self._last_fire.get(cooldown_key, 0.0)

        # Honour confidence floor for any non-noop action.
        if decision.action != "noop" and decision.confidence < self.min_confidence:
            return "recommend"

        # Cooldown blocks repeats of any actuating tool (noop is exempt).
        if decision.action != "noop" and (now - last) < self.cooldown_secs:
            return "blocked"

        exec_mode: ExecMode
        if self.mode == "read_only":
            exec_mode = "recommend"
        elif self.mode == "full_auto":
            exec_mode = "auto"
        else:  # low_risk_auto
            exec_mode = "auto" if decision.action in LOW_RISK_ACTIONS else "recommend"

        if exec_mode == "auto" and decision.action != "noop":
            self._last_fire[cooldown_key] = now
        return exec_mode
