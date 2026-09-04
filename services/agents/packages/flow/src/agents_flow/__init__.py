"""flow-agent (§10.1). Order-flow/microstructure analysis, T1 cadence.
Phase 3/5 scope (needs DOM data from a live MT5 feed)."""

from agents_core import AgentInput, AgentOutput, BaseAgent


class FlowAgent(BaseAgent):
    kind = "flow-agent"

    def run(self, agent_input: AgentInput) -> AgentOutput:
        raise NotImplementedError("FlowAgent is Phase 3/5 scope")
