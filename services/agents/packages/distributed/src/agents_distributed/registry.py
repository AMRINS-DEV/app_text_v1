"""Maps an `agent_kind` string to a factory that builds a fresh,
independently-constructed agent — called anew inside *each* worker process,
never shared or pickled across the process boundary. This is what makes the
distribution genuinely stateless: two different workers handling two
different `"regime-agent"` jobs never share a classifier object, yet
(being deterministically seeded from nothing but a fixed constant) compute
bit-identical decisions to a single in-process run — verified directly in
this package's own tests.

Only `regime-agent` is registered here (not the LLM-backed news/critic
agents from Phase 5): those need a real or mocked LLM transport, which is
an orthogonal concern to proving the distribution mechanism itself works,
and `RegimeAgent`'s `run` is genuinely synchronous logic under an `async`
signature (see `agents_core.agent.BaseAgent`'s own doc comment) — the
cleanest, most honest agent to distribute without smuggling in a second
kind of mock alongside this one.
"""

from __future__ import annotations

from collections.abc import Callable
from typing import Any

import numpy as np
from agents_core import AgentInput, BaseAgent
from agents_regime import RegimeAgent, RegimeAgentInput, RegimeClassifier

AgentFactory = Callable[[], BaseAgent]
InputBuilder = Callable[[dict[str, Any]], AgentInput]

# Fixed, deterministic synthetic training data + seed: every worker process
# that builds a regime-agent fits the *same* classifier from *nothing but
# this constant* -- no shared memory, no pickled model, no coordination
# between workers required for them to agree.
_TRAINING_SEED = 42
_N_TRAINING_BARS = 400


def _build_regime_agent() -> BaseAgent:
    rng = np.random.default_rng(_TRAINING_SEED)
    returns = rng.normal(0.0, 0.001, size=_N_TRAINING_BARS)
    volatility = np.abs(rng.normal(0.01, 0.002, size=_N_TRAINING_BARS))
    features = np.column_stack([returns, volatility])
    classifier = RegimeClassifier(n_states=4, random_state=_TRAINING_SEED).fit(features)
    return RegimeAgent(classifier)


AGENT_FACTORIES: dict[str, AgentFactory] = {
    "regime-agent": _build_regime_agent,
}

INPUT_BUILDERS: dict[str, InputBuilder] = {
    "regime-agent": lambda payload: RegimeAgentInput(**payload),
}
