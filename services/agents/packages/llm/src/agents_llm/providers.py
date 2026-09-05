"""Real `LlmProvider` adapters over each provider's actual HTTP API (§10.2).

None of these are called with a real API key anywhere in this repo's tests
or CI — every test injects a fake `httpx.AsyncClient` (via `httpx.MockTransport`)
that asserts the *request* this code builds matches the real API's documented
shape, then returns a canned *response* in that same API's real shape for
this code to parse. That is a genuine test of the integration logic (URL,
headers, payload, response parsing) without spending real money or needing
real credentials — the same "real logic, mocked infrastructure" split as
`SimBroker`/`InMemoryCoreClient` elsewhere in this repo. Wiring a real key
in is an environment-variable away (`OPENAI_API_KEY`, `ANTHROPIC_API_KEY`,
`DEEPSEEK_API_KEY`, `MOONSHOT_API_KEY`) but never exercised here.
"""

from __future__ import annotations

import time
from collections.abc import Callable

import httpx

from .provider import Caps, CompletionRequest, CompletionResponse, Cost, Health


class HttpLlmProvider:
    """Shared request/response bookkeeping every provider adapter needs:
    an injectable `httpx.AsyncClient` (so tests never touch the real
    network) and a real circuit breaker (§10.2: "3 consecutive
    failures/timeouts -> mark unhealthy for 60s, route to fallback") —
    `clock` is injectable too, so tests can simulate the 60s cooldown
    elapsing without an actual `time.sleep(60)`."""

    name: str = "unknown"
    COOLDOWN_SECONDS = 60.0
    FAILURE_THRESHOLD = 3

    def __init__(
        self,
        *,
        api_key: str | None = None,
        client: httpx.AsyncClient | None = None,
        clock: Callable[[], float] = time.time,
    ) -> None:
        self._api_key = api_key
        self._client = client or httpx.AsyncClient()
        self._clock = clock
        self._consecutive_failures = 0
        self._last_failure_at: float | None = None

    def health(self) -> Health:
        tripped = self._consecutive_failures >= self.FAILURE_THRESHOLD
        cooled_down = (
            tripped
            and self._last_failure_at is not None
            and (self._clock() - self._last_failure_at) >= self.COOLDOWN_SECONDS
        )
        return Health(
            healthy=(not tripped) or cooled_down, consecutive_failures=self._consecutive_failures
        )

    def _record_success(self) -> None:
        self._consecutive_failures = 0
        self._last_failure_at = None

    def _record_failure(self) -> None:
        self._consecutive_failures += 1
        self._last_failure_at = self._clock()


class OpenAIProvider(HttpLlmProvider):
    """GPT family via the Chat Completions API — strong tool use, structured
    output (§10.2)."""

    name = "openai"
    BASE_URL = "https://api.openai.com/v1/chat/completions"
    MODEL = "gpt-4o"

    def capabilities(self) -> Caps:
        return Caps(vision=True, tools=True, ctx_len=128_000, json_mode=True)

    def cost_per_mtok(self) -> Cost:
        return Cost(input_per_mtok_usd=2.50, output_per_mtok_usd=10.00)

    async def complete(self, req: CompletionRequest) -> CompletionResponse:
        payload: dict[str, object] = {
            "model": self.MODEL,
            "messages": [{"role": "user", "content": req.prompt}],
            "max_tokens": req.max_tokens,
        }
        if req.json_mode:
            payload["response_format"] = {"type": "json_object"}
        try:
            resp = await self._client.post(
                self.BASE_URL,
                headers={
                    "Authorization": f"Bearer {self._api_key}",
                    "Content-Type": "application/json",
                },
                json=payload,
            )
            resp.raise_for_status()
        except httpx.HTTPError:
            self._record_failure()
            raise
        self._record_success()
        body = resp.json()
        choice = body["choices"][0]["message"]["content"]
        usage = body["usage"]
        return CompletionResponse(
            text=choice,
            input_tokens=usage["prompt_tokens"],
            output_tokens=usage["completion_tokens"],
            model=body["model"],
        )


class AnthropicProvider(HttpLlmProvider):
    """Claude via the Messages API — long context, best reasoning/critique,
    prompt caching (§10.2)."""

    name = "claude"
    BASE_URL = "https://api.anthropic.com/v1/messages"
    MODEL = "claude-sonnet-5"
    API_VERSION = "2023-06-01"

    def capabilities(self) -> Caps:
        return Caps(vision=True, tools=True, ctx_len=200_000, json_mode=False)

    def cost_per_mtok(self) -> Cost:
        return Cost(input_per_mtok_usd=3.00, output_per_mtok_usd=15.00)

    async def complete(self, req: CompletionRequest) -> CompletionResponse:
        payload = {
            "model": self.MODEL,
            "max_tokens": req.max_tokens,
            "messages": [{"role": "user", "content": req.prompt}],
        }
        try:
            resp = await self._client.post(
                self.BASE_URL,
                headers={
                    "x-api-key": self._api_key or "",
                    "anthropic-version": self.API_VERSION,
                    "Content-Type": "application/json",
                },
                json=payload,
            )
            resp.raise_for_status()
        except httpx.HTTPError:
            self._record_failure()
            raise
        self._record_success()
        body = resp.json()
        text = "".join(block["text"] for block in body["content"] if block["type"] == "text")
        usage = body["usage"]
        return CompletionResponse(
            text=text,
            input_tokens=usage["input_tokens"],
            output_tokens=usage["output_tokens"],
            model=body["model"],
        )


class DeepSeekProvider(HttpLlmProvider):
    """Very cheap reasoning — bulk triage, backtest narration (§10.2). The
    API is OpenAI-Chat-Completions-compatible."""

    name = "deepseek"
    BASE_URL = "https://api.deepseek.com/chat/completions"
    MODEL = "deepseek-chat"

    def capabilities(self) -> Caps:
        return Caps(vision=False, tools=True, ctx_len=64_000, json_mode=True)

    def cost_per_mtok(self) -> Cost:
        return Cost(input_per_mtok_usd=0.14, output_per_mtok_usd=0.28)

    async def complete(self, req: CompletionRequest) -> CompletionResponse:
        payload: dict[str, object] = {
            "model": self.MODEL,
            "messages": [{"role": "user", "content": req.prompt}],
            "max_tokens": req.max_tokens,
        }
        if req.json_mode:
            payload["response_format"] = {"type": "json_object"}
        try:
            resp = await self._client.post(
                self.BASE_URL,
                headers={
                    "Authorization": f"Bearer {self._api_key}",
                    "Content-Type": "application/json",
                },
                json=payload,
            )
            resp.raise_for_status()
        except httpx.HTTPError:
            self._record_failure()
            raise
        self._record_success()
        body = resp.json()
        choice = body["choices"][0]["message"]["content"]
        usage = body["usage"]
        return CompletionResponse(
            text=choice,
            input_tokens=usage["prompt_tokens"],
            output_tokens=usage["completion_tokens"],
            model=body["model"],
        )


class KimiProvider(HttpLlmProvider):
    """Moonshot AI's Kimi — very long context, full-session/document
    analysis (§10.2). Also OpenAI-Chat-Completions-compatible."""

    name = "kimi"
    BASE_URL = "https://api.moonshot.ai/v1/chat/completions"
    MODEL = "moonshot-v1-256k"

    def capabilities(self) -> Caps:
        return Caps(vision=False, tools=True, ctx_len=256_000, json_mode=True)

    def cost_per_mtok(self) -> Cost:
        return Cost(input_per_mtok_usd=2.00, output_per_mtok_usd=5.00)

    async def complete(self, req: CompletionRequest) -> CompletionResponse:
        payload: dict[str, object] = {
            "model": self.MODEL,
            "messages": [{"role": "user", "content": req.prompt}],
            "max_tokens": req.max_tokens,
        }
        if req.json_mode:
            payload["response_format"] = {"type": "json_object"}
        try:
            resp = await self._client.post(
                self.BASE_URL,
                headers={
                    "Authorization": f"Bearer {self._api_key}",
                    "Content-Type": "application/json",
                },
                json=payload,
            )
            resp.raise_for_status()
        except httpx.HTTPError:
            self._record_failure()
            raise
        self._record_success()
        body = resp.json()
        choice = body["choices"][0]["message"]["content"]
        usage = body["usage"]
        return CompletionResponse(
            text=choice,
            input_tokens=usage["prompt_tokens"],
            output_tokens=usage["completion_tokens"],
            model=body["model"],
        )
