"""pattern-agent (§10.1, §12.3). The detection logic itself is deterministic
(no LLM) — only the narrative explanation optionally calls a model.
"""

from __future__ import annotations

from typing import Literal

import numpy as np
from agents_core import AgentInput, AgentOutput, BaseAgent
from agents_llm import CompletionRequest, LlmRouter

from .geometry import PatternInstance, SwingPoint, detect_double_top_or_bottom, find_swing_points

__all__ = [
    "PatternInstance",
    "SwingPoint",
    "detect_double_top_or_bottom",
    "find_swing_points",
    "PatternAgent",
    "PatternAgentInput",
]


class PatternAgentInput(AgentInput):
    highs: list[float]
    lows: list[float]
    regime: Literal["Trending", "Ranging", "Expansion", "HighVolChoppy"] = "Ranging"
    """Supplied by the orchestrator from regime-agent's own output (§10.1's
    roster is composed, not each agent re-deriving what another already
    computed) — defaults to "Ranging" only so this agent is usable
    standalone in tests without wiring a real regime-agent alongside it."""


_NO_PATTERN_OUTPUT = AgentOutput(
    direction="Flat",
    probability=0.5,
    confidence=0.0,
    expected_r=0.0,
    horizon_ms=0,
    regime="Ranging",
)


class PatternAgent(BaseAgent):
    kind = "pattern-agent"

    def __init__(
        self, *, router: LlmRouter | None = None, window: int = 3, horizon_ms: int = 3_600_000
    ) -> None:
        """`router=None` (the default) skips the narrative call entirely —
        detection and the resulting `AgentOutput` are fully real either
        way; the narrative is prose *about* an already-computed pattern,
        never something the numeric fields depend on."""
        self._router = router
        self._window = window
        self._horizon_ms = horizon_ms

    async def run(self, agent_input: AgentInput) -> AgentOutput:
        if not isinstance(agent_input, PatternAgentInput):
            msg = "PatternAgent requires a PatternAgentInput (highs/lows)"
            raise TypeError(msg)

        highs = np.array(agent_input.highs, dtype=float)
        lows = np.array(agent_input.lows, dtype=float)
        pattern = detect_double_top_or_bottom(highs, lows, window=self._window)
        if pattern is None:
            return _NO_PATTERN_OUTPUT.model_copy(update={"regime": agent_input.regime})

        direction: Literal["Long", "Short"] = "Short" if pattern.kind == "double_top" else "Long"
        expected_r = abs(pattern.target_price - pattern.neckline_price) / max(
            abs(pattern.invalidation_price - pattern.neckline_price), 1e-9
        )
        return AgentOutput(
            direction=direction,
            probability=0.5 + 0.4 * pattern.confidence,
            confidence=pattern.confidence,
            expected_r=expected_r,
            horizon_ms=self._horizon_ms,
            regime=agent_input.regime,
        )

    async def narrative(self, pattern: PatternInstance) -> str:
        """A plain-English explanation of an already-detected pattern
        (§10.1: "LLM only for narrative"). Raises if this agent was built
        without a router — there is nothing sensible to fall back to for
        prose generation, unlike the numeric detection above."""
        if self._router is None:
            msg = "PatternAgent was constructed without a router; narrative() is unavailable"
            raise RuntimeError(msg)
        prompt = (
            f"Describe this {pattern.kind.replace('_', ' ')} pattern in one sentence for a trader: "
            f"peak price {pattern.peak_price:.5f}, neckline {pattern.neckline_price:.5f}, "
            f"measured-move target {pattern.target_price:.5f}."
        )
        response = await self._router.complete(
            "pattern_narrative", CompletionRequest(prompt=prompt, max_tokens=120)
        )
        return response.text
