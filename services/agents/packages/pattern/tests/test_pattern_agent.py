from __future__ import annotations

import httpx
import numpy as np
import pytest
from agents_core import AgentInput
from agents_llm import DeepSeekProvider, LlmRouter
from agents_pattern import PatternAgent, PatternAgentInput
from agents_pattern.geometry import detect_double_top_or_bottom


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


@pytest.mark.anyio
async def test_pattern_agent_maps_a_double_top_to_a_short_signal():
    agent = PatternAgent()
    prices = _double_top_prices()
    agent_input = PatternAgentInput(symbol_id=1, as_of_ns=0, highs=prices, lows=prices)
    out = await agent.run(agent_input)
    assert out.direction == "Short"
    assert out.confidence > 0.9


@pytest.mark.anyio
async def test_pattern_agent_reports_flat_zero_confidence_when_no_pattern_found():
    agent = PatternAgent()
    prices = [100.0 + i for i in range(40)]  # monotonic ramp, no repeated extrema
    agent_input = PatternAgentInput(symbol_id=1, as_of_ns=0, highs=prices, lows=prices)
    out = await agent.run(agent_input)
    assert out.direction == "Flat"
    assert out.confidence == 0.0


@pytest.mark.anyio
async def test_pattern_agent_carries_the_supplied_regime_through():
    agent = PatternAgent()
    prices = _double_top_prices()
    agent_input = PatternAgentInput(
        symbol_id=1, as_of_ns=0, highs=prices, lows=prices, regime="HighVolChoppy"
    )
    out = await agent.run(agent_input)
    assert out.regime == "HighVolChoppy"


@pytest.mark.anyio
async def test_pattern_agent_rejects_a_plain_agent_input():
    agent = PatternAgent()
    with pytest.raises(TypeError):
        await agent.run(AgentInput(symbol_id=1, as_of_ns=0))


@pytest.mark.anyio
async def test_narrative_raises_without_a_router():
    agent = PatternAgent()
    prices = np.array(_double_top_prices())
    pattern = detect_double_top_or_bottom(prices, prices)
    assert pattern is not None
    with pytest.raises(RuntimeError):
        await agent.narrative(pattern)


@pytest.mark.anyio
async def test_narrative_calls_the_router_and_returns_its_text():
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(
            200,
            json={
                "model": "deepseek-chat",
                "choices": [{"message": {"content": "A double top formed near 110."}}],
                "usage": {"prompt_tokens": 5, "completion_tokens": 5},
            },
        )

    client = httpx.AsyncClient(transport=httpx.MockTransport(handler))
    router = LlmRouter(providers={"deepseek": DeepSeekProvider(client=client)})
    agent = PatternAgent(router=router)
    prices = np.array(_double_top_prices())
    pattern = detect_double_top_or_bottom(prices, prices)
    assert pattern is not None

    text = await agent.narrative(pattern)
    assert text == "A double top formed near 110."
    assert len(router.audit_log) == 1
    assert router.audit_log[0].task_class == "pattern_narrative"


@pytest.fixture
def anyio_backend():
    return "asyncio"
