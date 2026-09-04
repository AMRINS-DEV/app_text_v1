"""Provider adapters. Deliberately interface-complete stubs: no HTTP calls,
no API keys read, nothing that could incur cost or need credentials. Wiring
a real call is Phase 5 scope, gated by the cost/latency guardrails in
§10.2/§10.4 (hard monthly caps, per-call budget, structured-output
enforcement) landing first — never call a real provider before those exist.
"""

from __future__ import annotations

from .provider import Caps, CompletionRequest, CompletionResponse, Cost, Health


class _StubProvider:
    name: str = "stub"

    async def complete(self, req: CompletionRequest) -> CompletionResponse:
        raise NotImplementedError(f"{self.name} provider is Phase 5 scope (no real API calls yet)")

    def capabilities(self) -> Caps:
        raise NotImplementedError(f"{self.name} provider is Phase 5 scope")

    def cost_per_mtok(self) -> Cost:
        raise NotImplementedError(f"{self.name} provider is Phase 5 scope")

    def health(self) -> Health:
        return Health(healthy=False, consecutive_failures=0)


class OpenAIProvider(_StubProvider):
    """GPT family — strong tool use, structured output (§10.2)."""

    name = "openai"


class AnthropicProvider(_StubProvider):
    """Long context, best reasoning/critique, prompt caching (§10.2)."""

    name = "claude"


class DeepSeekProvider(_StubProvider):
    """Very cheap reasoning — bulk triage, backtest narration (§10.2)."""

    name = "deepseek"


class KimiProvider(_StubProvider):
    """Very long context — full-session/document analysis (§10.2)."""

    name = "kimi"
