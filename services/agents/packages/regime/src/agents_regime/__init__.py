"""regime-agent (§10.1). No LLM — HMM/GBDT classifier over the feature
vector produced by `crates/features`. Phase 3/5 scope (needs a trained
classifier from `agents-models`)."""

from agents_core import AgentInput, AgentOutput, BaseAgent


class RegimeAgent(BaseAgent):
    kind = "regime-agent"

    def run(self, agent_input: AgentInput) -> AgentOutput:
        raise NotImplementedError("RegimeAgent is Phase 3/5 scope")
