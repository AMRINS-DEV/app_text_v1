"""Routing policy table, verbatim from design doc §10.2. The router's
responsibilities (capability matching, cost/latency budgets, circuit
breaking, semantic caching, structured-output enforcement, full audit
trail) are Phase 5 scope — this fixes the policy shape now so the
provider stubs in `providers.py` have a concrete contract to be selected
against.
"""

from __future__ import annotations

from dataclasses import dataclass, field

ROUTING_POLICY: dict[str, dict[str, object]] = {
    "news_triage": {"primary": "deepseek", "fallback": ["kimi", "openai"], "max_latency_s": 3},
    "news_deep": {"primary": "claude", "fallback": ["openai"], "max_latency_s": 20},
    "chart_vision": {"primary": "openai", "fallback": ["claude"], "requires": ["vision"]},
    "critic": {"primary": "claude", "fallback": ["openai"], "max_latency_s": 10},
    "session_review": {"primary": "kimi", "fallback": ["claude"], "requires": ["ctx>200k"]},
    "pattern_narrative": {"primary": "deepseek", "fallback": ["kimi"]},
}


@dataclass
class LlmRouter:
    """Phase 0: holds the policy table. Selecting a healthy provider under
    budget, semantic-cache lookup (L3a) and circuit breaking are Phase 5 scope."""

    policy: dict[str, dict[str, object]] = field(default_factory=lambda: dict(ROUTING_POLICY))

    def route(self, task_class: str) -> str:
        try:
            return str(self.policy[task_class]["primary"])
        except KeyError as e:
            raise ValueError(f"no routing policy for task class {task_class!r}") from e
