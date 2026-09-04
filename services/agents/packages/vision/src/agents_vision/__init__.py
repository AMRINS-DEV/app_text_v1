"""vision-agent (§10.1). Requires a vision-capable frontier model (§10.2
`chart_vision` routing) and chart snapshot rendering — Phase 5 scope."""

from agents_core import AgentInput, AgentOutput, BaseAgent


class VisionAgent(BaseAgent):
    kind = "vision-agent"

    def run(self, agent_input: AgentInput) -> AgentOutput:
        raise NotImplementedError("VisionAgent is Phase 5 scope")
