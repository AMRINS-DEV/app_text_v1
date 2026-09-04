"""regime-agent (§10.1). No LLM — a real Gaussian HMM classifier
(`hmm.RegimeClassifier`) over a (return, realized-volatility) feature
history.
"""

from __future__ import annotations

import numpy as np
from agents_core import AgentInput, AgentOutput, BaseAgent

from .hmm import REGIME_LABELS, RegimeClassifier

__all__ = ["REGIME_LABELS", "RegimeClassifier", "RegimeAgent", "RegimeAgentInput"]


class RegimeAgentInput(AgentInput):
    """(return, realized_vol) pairs per bar, oldest first — the same
    feature pair `RegimeClassifier` was fit on."""

    feature_history: list[tuple[float, float]]


class RegimeAgent(BaseAgent):
    """Regime classification carries no directional view of its own
    (§10.1: "regime label + posterior probabilities" only) — `AgentOutput`
    is nonetheless the one shape every agent emits (§10.3), so `direction`
    is always `"Flat"` and `expected_r` is always `0.0` here; `probability`
    and `confidence` both carry the HMM's posterior for the classified
    regime, since that posterior *is* this agent's entire output."""

    kind = "regime-agent"

    def __init__(self, classifier: RegimeClassifier, *, horizon_ms: int = 60_000) -> None:
        self._classifier = classifier
        self._horizon_ms = horizon_ms

    async def run(self, agent_input: AgentInput) -> AgentOutput:
        if not isinstance(agent_input, RegimeAgentInput):
            msg = "RegimeAgent requires a RegimeAgentInput (feature_history)"
            raise TypeError(msg)
        features = np.array(agent_input.feature_history, dtype=float)
        regime, posterior = self._classifier.classify_latest(features)
        return AgentOutput(
            direction="Flat",
            probability=posterior,
            confidence=posterior,
            expected_r=0.0,
            horizon_ms=self._horizon_ms,
            regime=regime,  # type: ignore[arg-type]
        )
