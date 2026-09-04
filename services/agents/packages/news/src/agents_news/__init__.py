"""news-agent (§10.1): RSS/API feeds + econ calendar -> structured event
(type, instruments, expected direction, impact score). Fast/cheap model
tier for triage, frontier tier for high-impact events (§10.2's
`news_triage`/`news_deep` routing policy). Graph ingest (§7's NewsEvent
node) is Phase 6 scope, once FalkorDB exists — this agent's job stops at
producing the calibrated `AgentOutput`.
"""

from __future__ import annotations

from typing import Literal

from agents_core import AgentInput, AgentOutput, BaseAgent, implausible_levels, wrap_untrusted_text
from agents_llm import CompletionRequest, LlmRouter, parse_structured
from pydantic import BaseModel, Field

__all__ = ["NewsAgent", "NewsAgentInput", "NewsEventOutput"]

_DEEP_ANALYSIS_IMPACT_THRESHOLD = 0.7


class NewsEventOutput(BaseModel):
    """The structured shape the model must produce (§10.2's "structured
    output enforcement via Pydantic schemas + retry-with-repair")."""

    event_type: str
    instruments: list[str]
    expected_direction: Literal["Long", "Short", "Flat"]
    impact_score: float = Field(ge=0.0, le=1.0)
    numeric_levels: list[float] = Field(default_factory=list)


class NewsAgentInput(AgentInput):
    text: str
    source: str
    recent_low: float
    recent_high: float
    atr: float
    regime: Literal["Trending", "Ranging", "Expansion", "HighVolChoppy"] = "Ranging"


_DISCARDED_OUTPUT = AgentOutput(
    direction="Flat",
    probability=0.5,
    confidence=0.0,
    expected_r=0.0,
    horizon_ms=0,
    regime="Ranging",
)


class NewsAgent(BaseAgent):
    kind = "news-agent"

    def __init__(
        self, router: LlmRouter, *, horizon_ms: int = 3_600_000, max_repairs: int = 1
    ) -> None:
        self._router = router
        self._horizon_ms = horizon_ms
        self._max_repairs = max_repairs

    async def run(self, agent_input: AgentInput) -> AgentOutput:
        if not isinstance(agent_input, NewsAgentInput):
            msg = "NewsAgent requires a NewsAgentInput (text/source/recent_low/recent_high/atr)"
            raise TypeError(msg)

        # §10.2: fast/cheap triage by default; only genuinely high-impact
        # items go to the frontier tier. Impact is itself model output, so
        # the first pass always triages, and a high triage impact_score
        # triggers a second, deeper pass — never the reverse (never skip
        # straight to the expensive tier on an unverified guess).
        triage_event = await self._complete(agent_input, "news_triage")
        event = (
            await self._complete(agent_input, "news_deep")
            if triage_event.impact_score >= _DEEP_ANALYSIS_IMPACT_THRESHOLD
            else triage_event
        )

        # §10.4: "all numeric levels are cross-checked against actual
        # OHLCV; mismatch > 0.1 ATR -> discard signal."
        if event.numeric_levels:
            bad = implausible_levels(
                event.numeric_levels,
                recent_low=agent_input.recent_low,
                recent_high=agent_input.recent_high,
                atr=agent_input.atr,
            )
            if bad:
                return _DISCARDED_OUTPUT.model_copy(update={"regime": agent_input.regime})

        probability = 0.5 + 0.4 * event.impact_score if event.expected_direction != "Flat" else 0.5
        return AgentOutput(
            direction=event.expected_direction,
            probability=probability,
            confidence=event.impact_score,
            expected_r=event.impact_score,
            horizon_ms=self._horizon_ms,
            regime=agent_input.regime,
        )

    async def _complete(self, agent_input: NewsAgentInput, task_class: str) -> NewsEventOutput:
        prompt = self._build_prompt(agent_input)

        async def repair(bad_text: str, error: str) -> str:
            repair_prompt = (
                "Your previous reply could not be parsed as the required JSON schema "
                f"(error: {error}). Reply again with ONLY the corrected JSON object, "
                f"no other text. Previous reply:\n{bad_text}"
            )
            response = await self._router.complete(
                task_class, CompletionRequest(prompt=repair_prompt, json_mode=True), agent=self.kind
            )
            return response.text

        response = await self._router.complete(
            task_class, CompletionRequest(prompt=prompt, json_mode=True), agent=self.kind
        )
        return await parse_structured(
            response.text, NewsEventOutput, repair=repair, max_repairs=self._max_repairs
        )

    def _build_prompt(self, agent_input: NewsAgentInput) -> str:
        schema = (
            '{"event_type": string, "instruments": [string], '
            '"expected_direction": "Long" | "Short" | "Flat", '
            '"impact_score": number in [0,1], "numeric_levels": [number]}'
        )
        return (
            "You are a financial news triage analyst. Analyze the news item below and "
            f"reply with ONLY a JSON object matching this schema: {schema}\n\n"
            f"{wrap_untrusted_text(agent_input.text, source=agent_input.source)}"
        )
