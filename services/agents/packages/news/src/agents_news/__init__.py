"""news-agent (§10.1): RSS/API feeds + econ calendar -> structured event
(type, instruments, expected direction, impact score, embedding). Fast/cheap
model tier for triage, frontier tier for high-impact events (§10.2's
`news_triage`/`news_deep` routing policy). Phase 5 scope."""

from agents_core import AgentInput, AgentOutput, BaseAgent


class NewsAgent(BaseAgent):
    kind = "news-agent"

    def run(self, agent_input: AgentInput) -> AgentOutput:
        raise NotImplementedError("NewsAgent is Phase 5 scope")
