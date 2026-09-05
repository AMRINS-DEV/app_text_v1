from __future__ import annotations

import json

import httpx
import numpy as np
import pytest
from agents_core import SignalBus
from agents_critic import CriticAgent
from agents_llm import DeepSeekProvider, LlmRouter
from agents_news import NewsAgent, NewsAgentInput
from agents_orchestrator import AGENT_ROSTER, BarContext, Orchestrator
from agents_pattern import PatternAgent, PatternAgentInput
from agents_regime import RegimeAgent, RegimeAgentInput, RegimeClassifier


def _double_top_prices() -> list[float]:
    def ramp(start: float, end: float, n: int) -> list[float]:
        step = (end - start) / (n - 1)
        return [start + step * i for i in range(n)]

    return (
        ramp(100, 110, 10)
        + ramp(110, 100, 10)[1:]
        + ramp(100, 110, 10)[1:]
        + ramp(110, 95, 10)[1:]
    )


def _fitted_regime_classifier() -> RegimeClassifier:
    rng = np.random.default_rng(11)
    trending = np.column_stack(
        [rng.normal(0.0020, 0.0004, 200), np.abs(rng.normal(0.0006, 0.0001, 200))]
    )
    ranging = np.column_stack(
        [rng.normal(0.0, 0.0002, 200), np.abs(rng.normal(0.0003, 0.00005, 200))]
    )
    expansion = np.column_stack(
        [rng.normal(0.0, 0.0010, 200), np.abs(rng.normal(0.0012, 0.0002, 200))]
    )
    choppy = np.column_stack(
        [rng.normal(0.0, 0.0020, 200), np.abs(rng.normal(0.0025, 0.0003, 200))]
    )
    features = np.concatenate([trending, ranging, expansion, choppy])
    return RegimeClassifier(n_states=4, random_state=11).fit(features)


def _json_response(payload: dict) -> httpx.Response:
    return httpx.Response(
        200,
        json={
            "model": "test-model",
            "choices": [{"message": {"content": json.dumps(payload)}}],
            "usage": {"prompt_tokens": 5, "completion_tokens": 5},
        },
    )


def _critic_router(*, approve: bool) -> LlmRouter:
    def handler(request: httpx.Request) -> httpx.Response:
        return _json_response(
            {"approve": approve, "reasoning": "test", "adjusted_confidence": 0.75}
        )

    client = httpx.AsyncClient(transport=httpx.MockTransport(handler))
    return LlmRouter(providers={"claude": DeepSeekProvider(client=client)})


def _make_orchestrator(*, critic_approves: bool) -> Orchestrator:
    regime = RegimeAgent(_fitted_regime_classifier())
    pattern = PatternAgent()
    critic = CriticAgent(_critic_router(approve=critic_approves))
    return Orchestrator(regime=regime, pattern=pattern, critic=critic, signal_bus=SignalBus())


def _bar_context(*, orchestrator: Orchestrator) -> BarContext:
    prices = _double_top_prices()
    orchestrator._signal_bus.register_feature_snapshot("hash-1")
    return BarContext(
        symbol_id=1,
        as_of_ns=1_000,
        features_hash="hash-1",
        ttl_ns=60_000,
        regime_input=RegimeAgentInput(
            symbol_id=1, as_of_ns=1_000, feature_history=[(0.0020, 0.0006)] * 30
        ),
        pattern_input=PatternAgentInput(symbol_id=1, as_of_ns=1_000, highs=prices, lows=prices),
    )


def test_agent_roster_only_lists_agents_with_a_real_implementation():
    assert set(AGENT_ROSTER) == {"news-agent", "pattern-agent", "regime-agent", "critic-agent"}


@pytest.mark.anyio
async def test_a_pattern_signal_that_the_critic_approves_gets_published():
    orchestrator = _make_orchestrator(critic_approves=True)
    ctx = _bar_context(orchestrator=orchestrator)

    published = await orchestrator.process_bar(ctx)

    assert len(published) == 1
    assert published[0].agent_kind == "pattern-agent"
    assert published[0].output.direction == "Short"  # the double top maps to Short
    assert orchestrator._signal_bus.active_signals(now_ns=1_000) == published


@pytest.mark.anyio
async def test_a_pattern_signal_that_the_critic_vetoes_is_never_published():
    orchestrator = _make_orchestrator(critic_approves=False)
    ctx = _bar_context(orchestrator=orchestrator)

    published = await orchestrator.process_bar(ctx)

    assert published == []
    assert orchestrator._signal_bus.active_signals(now_ns=1_000) == []
    # The veto is still tracked (§10.1's outcome measurement), even though nothing published.
    assert len(orchestrator._critic.outcome_tracker._records) == 1


@pytest.mark.anyio
async def test_no_pattern_found_means_the_critic_is_never_even_consulted():
    regime = RegimeAgent(_fitted_regime_classifier())
    pattern = PatternAgent()
    critic_calls = {"n": 0}

    def handler(request: httpx.Request) -> httpx.Response:
        critic_calls["n"] += 1
        return _json_response({"approve": True, "reasoning": "x", "adjusted_confidence": 0.5})

    client = httpx.AsyncClient(transport=httpx.MockTransport(handler))
    critic = CriticAgent(LlmRouter(providers={"claude": DeepSeekProvider(client=client)}))
    signal_bus = SignalBus()
    signal_bus.register_feature_snapshot("hash-1")
    orchestrator = Orchestrator(
        regime=regime, pattern=pattern, critic=critic, signal_bus=signal_bus
    )

    monotonic_prices = [100.0 + i for i in range(40)]
    ctx = BarContext(
        symbol_id=1,
        as_of_ns=1_000,
        features_hash="hash-1",
        ttl_ns=60_000,
        regime_input=RegimeAgentInput(
            symbol_id=1, as_of_ns=1_000, feature_history=[(0.0020, 0.0006)] * 30
        ),
        pattern_input=PatternAgentInput(
            symbol_id=1, as_of_ns=1_000, highs=monotonic_prices, lows=monotonic_prices
        ),
    )

    published = await orchestrator.process_bar(ctx)

    assert published == []
    assert critic_calls["n"] == 0


@pytest.mark.anyio
async def test_news_input_is_processed_alongside_pattern_when_a_news_agent_is_wired_in():
    def news_handler(request: httpx.Request) -> httpx.Response:
        return _json_response(
            {
                "event_type": "rate_decision",
                "instruments": ["EURUSD"],
                "expected_direction": "Long",
                "impact_score": 0.2,
                "numeric_levels": [],
            }
        )

    news_client = httpx.AsyncClient(transport=httpx.MockTransport(news_handler))
    news = NewsAgent(LlmRouter(providers={"deepseek": DeepSeekProvider(client=news_client)}))

    regime = RegimeAgent(_fitted_regime_classifier())
    pattern = PatternAgent()
    critic = CriticAgent(_critic_router(approve=True))
    signal_bus = SignalBus()
    signal_bus.register_feature_snapshot("hash-1")
    orchestrator = Orchestrator(
        regime=regime, pattern=pattern, critic=critic, signal_bus=signal_bus, news=news
    )

    prices = _double_top_prices()
    ctx = BarContext(
        symbol_id=1,
        as_of_ns=1_000,
        features_hash="hash-1",
        ttl_ns=60_000,
        regime_input=RegimeAgentInput(
            symbol_id=1, as_of_ns=1_000, feature_history=[(0.0020, 0.0006)] * 30
        ),
        pattern_input=PatternAgentInput(symbol_id=1, as_of_ns=1_000, highs=prices, lows=prices),
        news_input=NewsAgentInput(
            symbol_id=1,
            as_of_ns=1_000,
            text="Rates cut by 50bps.",
            source="test-feed",
            recent_low=1.07,
            recent_high=1.09,
            atr=0.001,
        ),
    )

    published = await orchestrator.process_bar(ctx)

    kinds = {p.agent_kind for p in published}
    assert kinds == {"pattern-agent", "news-agent"}


@pytest.fixture
def anyio_backend():
    return "asyncio"
