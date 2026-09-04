"""Every agent's output is a `Signal`-shaped, schema-validated payload
(§10.3, §10.4) — never free text reaching the strategy VM. Agents have
zero order-placement authority; they only ever produce `AgentOutput`.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Literal

from pydantic import BaseModel, Field


class AgentInput(BaseModel):
    symbol_id: int
    as_of_ns: int


class AgentOutput(BaseModel):
    """Mirrors the calibrated fields of `domain::Signal` (Rust) /
    `packages/proto/signal.proto` — an agent never emits a raw score."""

    direction: Literal["Long", "Short", "Flat"]
    probability: float = Field(ge=0.0, le=1.0)
    confidence: float = Field(ge=0.0, le=1.0)
    expected_r: float
    horizon_ms: int = Field(ge=0)
    regime: Literal["Trending", "Ranging", "Expansion", "HighVolChoppy"]


class BaseAgent(ABC):
    """Subclasses implement `run`; publishing to NATS, isotonic calibration
    (§10.4) and the Brier-score weight tracker (§8.4) are handled by the
    orchestrator, not by individual agents.

    `run` is `async` uniformly across the whole roster, even for agents
    (regime, pattern) whose own logic never awaits anything — LLM-backed
    agents (news, critic) genuinely need to await a network call, and one
    interface both kinds of agent satisfy is simpler for the orchestrator
    than dispatching sync vs. async agents differently. A synchronous
    agent's `run` body just never has an `await` in it, which is valid."""

    kind: str

    @abstractmethod
    async def run(self, agent_input: AgentInput) -> AgentOutput: ...
