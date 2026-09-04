"""critic-agent (§10.1). Vetoes are logged and measured — if the critic's
vetoed trades would have been profitable, its weight is reduced
automatically (Phase 5 scope, needs the outcome ledger from §7/§14)."""

from agents_core import AgentInput, AgentOutput, BaseAgent


class CriticAgent(BaseAgent):
    kind = "critic-agent"

    def run(self, agent_input: AgentInput) -> AgentOutput:
        raise NotImplementedError("CriticAgent is Phase 5 scope")
