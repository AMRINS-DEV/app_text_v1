"""pattern-agent (§10.1, §12.3). The detection logic itself is deterministic
(no LLM) — only the narrative explanation calls a model. Phase 5/6 scope."""

from agents_core import AgentInput, AgentOutput, BaseAgent


class PatternAgent(BaseAgent):
    kind = "pattern-agent"

    def run(self, agent_input: AgentInput) -> AgentOutput:
        raise NotImplementedError("PatternAgent is Phase 5/6 scope")
