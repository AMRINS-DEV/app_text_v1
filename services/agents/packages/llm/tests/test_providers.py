"""Each provider's `complete()` is tested against a fake `httpx.AsyncClient`
that asserts the outgoing request matches that provider's real documented
API shape, then returns a response in that same real shape — a genuine
test of the integration logic without any real network call, key, or spend.
"""

from __future__ import annotations

import json

import httpx
import pytest
from agents_llm import (
    AnthropicProvider,
    CompletionRequest,
    DeepSeekProvider,
    KimiProvider,
    OpenAIProvider,
)


def client_for(handler):
    return httpx.AsyncClient(transport=httpx.MockTransport(handler))


@pytest.mark.anyio
async def test_openai_provider_builds_the_real_chat_completions_request():
    captured = {}

    def handler(request: httpx.Request) -> httpx.Response:
        captured["url"] = str(request.url)
        captured["auth"] = request.headers["authorization"]
        captured["body"] = json.loads(request.content)
        return httpx.Response(
            200,
            json={
                "model": "gpt-4o-2024-08-06",
                "choices": [{"message": {"content": "hello"}}],
                "usage": {"prompt_tokens": 10, "completion_tokens": 3},
            },
        )

    provider = OpenAIProvider(api_key="sk-test", client=client_for(handler))
    result = await provider.complete(CompletionRequest(prompt="hi", max_tokens=50))

    assert captured["url"] == "https://api.openai.com/v1/chat/completions"
    assert captured["auth"] == "Bearer sk-test"
    assert captured["body"]["messages"] == [{"role": "user", "content": "hi"}]
    assert result.text == "hello"
    assert result.input_tokens == 10
    assert result.output_tokens == 3
    assert result.model == "gpt-4o-2024-08-06"


@pytest.mark.anyio
async def test_openai_provider_sets_json_mode_response_format():
    def handler(request: httpx.Request) -> httpx.Response:
        body = json.loads(request.content)
        assert body["response_format"] == {"type": "json_object"}
        return httpx.Response(
            200,
            json={
                "model": "gpt-4o",
                "choices": [{"message": {"content": "{}"}}],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1},
            },
        )

    provider = OpenAIProvider(api_key="sk-test", client=client_for(handler))
    await provider.complete(CompletionRequest(prompt="hi", json_mode=True))


@pytest.mark.anyio
async def test_anthropic_provider_builds_the_real_messages_api_request():
    captured = {}

    def handler(request: httpx.Request) -> httpx.Response:
        captured["url"] = str(request.url)
        captured["api_key_header"] = request.headers["x-api-key"]
        captured["version_header"] = request.headers["anthropic-version"]
        return httpx.Response(
            200,
            json={
                "model": "claude-sonnet-5",
                "content": [{"type": "text", "text": "ack"}],
                "usage": {"input_tokens": 5, "output_tokens": 2},
            },
        )

    provider = AnthropicProvider(api_key="ak-test", client=client_for(handler))
    result = await provider.complete(CompletionRequest(prompt="hi"))

    assert captured["url"] == "https://api.anthropic.com/v1/messages"
    assert captured["api_key_header"] == "ak-test"
    assert captured["version_header"] == "2023-06-01"
    assert result.text == "ack"


@pytest.mark.anyio
async def test_anthropic_provider_concatenates_multiple_text_blocks():
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(
            200,
            json={
                "model": "claude-sonnet-5",
                "content": [{"type": "text", "text": "a"}, {"type": "text", "text": "b"}],
                "usage": {"input_tokens": 1, "output_tokens": 1},
            },
        )

    provider = AnthropicProvider(api_key="ak-test", client=client_for(handler))
    result = await provider.complete(CompletionRequest(prompt="hi"))
    assert result.text == "ab"


@pytest.mark.anyio
async def test_deepseek_provider_hits_the_real_endpoint():
    def handler(request: httpx.Request) -> httpx.Response:
        assert str(request.url) == "https://api.deepseek.com/chat/completions"
        return httpx.Response(
            200,
            json={
                "model": "deepseek-chat",
                "choices": [{"message": {"content": "cheap"}}],
                "usage": {"prompt_tokens": 2, "completion_tokens": 1},
            },
        )

    provider = DeepSeekProvider(api_key="ds-test", client=client_for(handler))
    result = await provider.complete(CompletionRequest(prompt="hi"))
    assert result.text == "cheap"


@pytest.mark.anyio
async def test_kimi_provider_hits_the_real_endpoint():
    def handler(request: httpx.Request) -> httpx.Response:
        assert str(request.url) == "https://api.moonshot.ai/v1/chat/completions"
        return httpx.Response(
            200,
            json={
                "model": "moonshot-v1-256k",
                "choices": [{"message": {"content": "long context"}}],
                "usage": {"prompt_tokens": 4, "completion_tokens": 2},
            },
        )

    provider = KimiProvider(api_key="km-test", client=client_for(handler))
    result = await provider.complete(CompletionRequest(prompt="hi"))
    assert result.text == "long context"


@pytest.mark.anyio
async def test_a_failed_call_is_tracked_toward_the_circuit_breaker():
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(500, json={"error": "boom"})

    provider = OpenAIProvider(api_key="sk-test", client=client_for(handler))
    with pytest.raises(httpx.HTTPError):
        await provider.complete(CompletionRequest(prompt="hi"))

    health = provider.health()
    assert health.consecutive_failures == 1
    assert health.healthy is True  # not yet at the 3-failure threshold


@pytest.mark.anyio
async def test_three_consecutive_failures_marks_the_provider_unhealthy():
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(500, json={"error": "boom"})

    provider = OpenAIProvider(api_key="sk-test", client=client_for(handler))
    for _ in range(3):
        with pytest.raises(httpx.HTTPError):
            await provider.complete(CompletionRequest(prompt="hi"))

    assert provider.health().healthy is False


@pytest.mark.anyio
async def test_a_success_resets_the_failure_count():
    calls = {"n": 0}

    def handler(request: httpx.Request) -> httpx.Response:
        calls["n"] += 1
        if calls["n"] == 1:
            return httpx.Response(500, json={"error": "boom"})
        return httpx.Response(
            200,
            json={
                "model": "gpt-4o",
                "choices": [{"message": {"content": "ok"}}],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1},
            },
        )

    provider = OpenAIProvider(api_key="sk-test", client=client_for(handler))
    with pytest.raises(httpx.HTTPError):
        await provider.complete(CompletionRequest(prompt="hi"))
    await provider.complete(CompletionRequest(prompt="hi"))

    assert provider.health().consecutive_failures == 0


@pytest.mark.anyio
async def test_circuit_breaker_recovers_after_the_cooldown_elapses():
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(500, json={"error": "boom"})

    fake_now = [1_000.0]
    provider = OpenAIProvider(
        api_key="sk-test", client=client_for(handler), clock=lambda: fake_now[0]
    )
    for _ in range(3):
        with pytest.raises(httpx.HTTPError):
            await provider.complete(CompletionRequest(prompt="hi"))
    assert provider.health().healthy is False

    fake_now[0] += 61.0  # past the 60s cooldown, no real sleep needed
    assert provider.health().healthy is True


@pytest.fixture
def anyio_backend():
    return "asyncio"
