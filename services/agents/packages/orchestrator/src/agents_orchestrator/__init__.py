"""Wires the agent roster together and publishes fused signals to the
`SignalBus` (§2, §10, §10.3). A plain async pipeline, not LangGraph's
stateful/checkpointed graph — LangGraph is a specific orchestration
*library*; the observable behavior §17's Phase 5 exit list actually asks
for ("news/pattern/regime/critic agents... signals published with TTL")
doesn't require it, and pulling in a graph-execution framework for a
four-node linear-with-one-fan-out pipeline would be a large dependency for
no behavior this pipeline doesn't already have. `AGENT_ROSTER` only lists
the agents with a real (non-stub) implementation — `vision-agent` and
`flow-agent` still raise `NotImplementedError` (Phase 5+ scope, pending a
chart-rendering pipeline and a live DOM feed respectively) and are
intentionally left out rather than wired in to fail at call time.
"""

from __future__ import annotations

from dataclasses import dataclass

from agents_core import AgentOutput, BaseAgent, PublishedSignal, SignalBus
from agents_critic import CriticAgent, CriticAgentInput
from agents_news import NewsAgent, NewsAgentInput
from agents_pattern import PatternAgent, PatternAgentInput
from agents_regime import RegimeAgent, RegimeAgentInput

AGENT_ROSTER: dict[str, type[BaseAgent]] = {
    "news-agent": NewsAgent,
    "pattern-agent": PatternAgent,
    "regime-agent": RegimeAgent,
    "critic-agent": CriticAgent,
}

__all__ = ["AGENT_ROSTER", "Orchestrator", "BarContext"]


@dataclass
class BarContext:
    """Everything one `Orchestrator.process_bar` call needs, gathered by
    the caller (the real deployment: `tradeos-core` on a bar close, over
    whatever bridge feeds this Python process)."""

    symbol_id: int
    as_of_ns: int
    features_hash: str
    ttl_ns: int
    regime_input: RegimeAgentInput
    pattern_input: PatternAgentInput
    news_input: NewsAgentInput | None = None


class Orchestrator:
    def __init__(
        self,
        *,
        regime: RegimeAgent,
        pattern: PatternAgent,
        critic: CriticAgent,
        signal_bus: SignalBus,
        news: NewsAgent | None = None,
    ) -> None:
        self._regime = regime
        self._pattern = pattern
        self._critic = critic
        self._news = news
        self._signal_bus = signal_bus

    async def process_bar(self, ctx: BarContext) -> list[PublishedSignal]:
        regime_output = await self._regime.run(ctx.regime_input)

        proposals: list[tuple[str, AgentOutput]] = []

        pattern_input = ctx.pattern_input.model_copy(update={"regime": regime_output.regime})
        proposals.append(("pattern-agent", await self._pattern.run(pattern_input)))

        if self._news is not None and ctx.news_input is not None:
            news_input = ctx.news_input.model_copy(update={"regime": regime_output.regime})
            proposals.append(("news-agent", await self._news.run(news_input)))

        published: list[PublishedSignal] = []
        for agent_kind, proposal in proposals:
            # A "Flat"/zero-confidence proposal is each agent's own
            # convention for "nothing to report" (§10.1) — never sent to
            # the critic, since there is nothing for it to evaluate.
            if proposal.direction == "Flat" or proposal.confidence <= 0.0:
                continue

            signal_id = f"{agent_kind}:{ctx.symbol_id}:{ctx.as_of_ns}"
            critic_input = CriticAgentInput(
                symbol_id=ctx.symbol_id,
                as_of_ns=ctx.as_of_ns,
                signal_id=signal_id,
                proposed_direction=proposal.direction,
                proposed_probability=proposal.probability,
                proposed_confidence=proposal.confidence,
                proposed_expected_r=proposal.expected_r,
                proposed_horizon_ms=proposal.horizon_ms,
                regime=proposal.regime,
                evidence=(
                    f"Regime-agent reports {regime_output.regime} "
                    f"(posterior {regime_output.probability:.2f})."
                ),
            )
            final = await self._critic.run(critic_input)
            if final.direction == "Flat":
                continue  # vetoed — the critic's own outcome tracker already logged it

            published_signal = PublishedSignal(
                symbol_id=ctx.symbol_id,
                agent_kind=agent_kind,
                output=final,
                features_hash=ctx.features_hash,
                published_at_ns=ctx.as_of_ns,
                ttl_ns=ctx.ttl_ns,
            )
            self._signal_bus.publish(published_signal, now_ns=ctx.as_of_ns)
            published.append(published_signal)

        return published
