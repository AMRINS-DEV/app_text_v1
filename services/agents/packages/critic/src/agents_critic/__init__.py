"""critic-agent (§10.1): "sees the proposal and the counter-evidence
(graph priors, recent similar-setup failures, upcoming news) and can
veto." The highest-leverage agent in the roster — it runs *before* a
signal publishes, per §10.3's contract.
"""

from __future__ import annotations

from typing import Literal

from agents_core import AgentInput, AgentOutput, BaseAgent, wrap_untrusted_text
from agents_llm import CompletionRequest, LlmRouter, parse_structured
from pydantic import BaseModel, Field

from .outcome_tracker import CriticOutcomeTracker, VetoRecord

__all__ = ["CriticAgent", "CriticAgentInput", "CriticVerdict", "CriticOutcomeTracker", "VetoRecord"]


class CriticVerdict(BaseModel):
    approve: bool
    reasoning: str
    adjusted_confidence: float = Field(ge=0.0, le=1.0)


class CriticAgentInput(AgentInput):
    signal_id: str
    proposed_direction: Literal["Long", "Short", "Flat"]
    proposed_probability: float = Field(ge=0.0, le=1.0)
    proposed_confidence: float = Field(ge=0.0, le=1.0)
    proposed_expected_r: float
    proposed_horizon_ms: int = Field(ge=0)
    regime: Literal["Trending", "Ranging", "Expansion", "HighVolChoppy"] = "Ranging"
    """Counter-evidence: graph priors, recent similar-setup failures,
    upcoming news — whatever the orchestrator gathered. Untrusted the same
    way news text is (§10.4) if any of it originates from scraped content."""
    evidence: str


class CriticAgent(BaseAgent):
    kind = "critic-agent"

    def __init__(
        self,
        router: LlmRouter,
        *,
        outcome_tracker: CriticOutcomeTracker | None = None,
        max_repairs: int = 1,
    ) -> None:
        self._router = router
        self._outcome_tracker = outcome_tracker or CriticOutcomeTracker()
        self._max_repairs = max_repairs

    @property
    def outcome_tracker(self) -> CriticOutcomeTracker:
        return self._outcome_tracker

    async def run(self, agent_input: AgentInput) -> AgentOutput:
        if not isinstance(agent_input, CriticAgentInput):
            msg = "CriticAgent requires a CriticAgentInput (proposed signal + evidence)"
            raise TypeError(msg)

        verdict = await self._verdict(agent_input)

        if not verdict.approve:
            self._outcome_tracker.record_veto(agent_input.signal_id, agent_input.as_of_ns)
            return AgentOutput(
                direction="Flat",
                probability=0.5,
                confidence=0.0,
                expected_r=0.0,
                horizon_ms=0,
                regime=agent_input.regime,
            )

        return AgentOutput(
            direction=agent_input.proposed_direction,
            probability=agent_input.proposed_probability,
            confidence=verdict.adjusted_confidence,
            expected_r=agent_input.proposed_expected_r,
            horizon_ms=agent_input.proposed_horizon_ms,
            regime=agent_input.regime,
        )

    async def _verdict(self, agent_input: CriticAgentInput) -> CriticVerdict:
        prompt = self._build_prompt(agent_input)

        async def repair(bad_text: str, error: str) -> str:
            repair_prompt = (
                "Your previous reply could not be parsed as the required JSON schema "
                f"(error: {error}). Reply again with ONLY the corrected JSON object, "
                f"no other text. Previous reply:\n{bad_text}"
            )
            response = await self._router.complete(
                "critic", CompletionRequest(prompt=repair_prompt, json_mode=True), agent=self.kind
            )
            return response.text

        response = await self._router.complete(
            "critic", CompletionRequest(prompt=prompt, json_mode=True), agent=self.kind
        )
        return await parse_structured(
            response.text, CriticVerdict, repair=repair, max_repairs=self._max_repairs
        )

    def _build_prompt(self, agent_input: CriticAgentInput) -> str:
        schema = '{"approve": boolean, "reasoning": string, "adjusted_confidence": number in [0,1]}'
        proposal = (
            f"Proposed signal: direction={agent_input.proposed_direction}, "
            f"probability={agent_input.proposed_probability:.3f}, "
            f"confidence={agent_input.proposed_confidence:.3f}, "
            f"expected_r={agent_input.proposed_expected_r:.3f}, regime={agent_input.regime}."
        )
        return (
            "You are a trading signal critic. Review the proposed signal against the "
            "counter-evidence below and decide whether to approve or veto it. Reply with "
            f"ONLY a JSON object matching this schema: {schema}\n\n"
            f"{proposal}\n\n"
            f"{wrap_untrusted_text(agent_input.evidence, source='counter-evidence')}"
        )
