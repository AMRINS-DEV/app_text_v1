"""Foundation for every agent (§10). Real for Phase 0: the `BaseAgent`
shape and the calibrated-signal contract every agent must emit. Memory,
eval harness, and tracing wiring are Phase 5 scope.
"""

from .agent import AgentInput, AgentOutput, BaseAgent

__all__ = ["AgentInput", "AgentOutput", "BaseAgent"]
