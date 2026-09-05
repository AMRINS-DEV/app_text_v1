from __future__ import annotations

import numpy as np
import pytest
from agents_core import AgentInput
from agents_regime import RegimeAgent, RegimeAgentInput, RegimeClassifier


def _column(
    rng: np.random.Generator, mean_r: float, std_r: float, mean_v: float, std_v: float
) -> np.ndarray:
    return np.column_stack([rng.normal(mean_r, std_r, 200), np.abs(rng.normal(mean_v, std_v, 200))])


def _fitted_classifier() -> RegimeClassifier:
    rng = np.random.default_rng(3)
    trending = _column(rng, 0.0020, 0.0004, 0.0006, 0.0001)
    ranging = _column(rng, 0.0, 0.0002, 0.0003, 0.00005)
    expansion = _column(rng, 0.0, 0.0010, 0.0012, 0.0002)
    choppy = _column(rng, 0.0, 0.0020, 0.0025, 0.0003)
    features = np.concatenate([trending, ranging, expansion, choppy])
    return RegimeClassifier(n_states=4, random_state=3).fit(features)


@pytest.mark.anyio
async def test_regime_agent_returns_a_flat_direction_and_zero_expected_r():
    agent = RegimeAgent(_fitted_classifier())
    trending_window = [(0.0020, 0.0006)] * 30
    agent_input = RegimeAgentInput(symbol_id=1, as_of_ns=0, feature_history=trending_window)
    out = await agent.run(agent_input)
    assert out.direction == "Flat"
    assert out.expected_r == 0.0
    assert out.regime in {"Trending", "Ranging", "Expansion", "HighVolChoppy"}


@pytest.mark.anyio
async def test_regime_agent_probability_and_confidence_both_carry_the_posterior():
    agent = RegimeAgent(_fitted_classifier())
    window = [(0.0020, 0.0006)] * 30
    agent_input = RegimeAgentInput(symbol_id=1, as_of_ns=0, feature_history=window)
    out = await agent.run(agent_input)
    assert out.probability == out.confidence


@pytest.mark.anyio
async def test_regime_agent_rejects_a_plain_agent_input():
    agent = RegimeAgent(_fitted_classifier())
    with pytest.raises(TypeError):
        await agent.run(AgentInput(symbol_id=1, as_of_ns=0))


@pytest.fixture
def anyio_backend():
    return "asyncio"
