"""Real dispatch behavior of `LlmRouter.complete()` — capability matching,
fallback on failure, circuit breaking, spend caps, semantic cache
integration, and the audit trail. Uses `httpx.MockTransport`-backed
providers throughout, never real network/keys.
"""

from __future__ import annotations

import httpx
import pytest
from agents_llm import (
    AllProvidersUnavailableError,
    AnthropicProvider,
    CompletionRequest,
    DeepSeekProvider,
    KimiProvider,
    LlmRouter,
    OpenAIProvider,
    SemanticCache,
    SpendCap,
)


def ok_client(text: str, model: str = "test-model") -> httpx.AsyncClient:
    def handler(request: httpx.Request) -> httpx.Response:
        # Both the OpenAI-compatible shape and the Anthropic shape need to
        # be servable from the same fake, since different providers parse
        # different response bodies.
        if "anthropic" in str(request.url):
            return httpx.Response(
                200,
                json={
                    "model": model,
                    "content": [{"type": "text", "text": text}],
                    "usage": {"input_tokens": 1, "output_tokens": 1},
                },
            )
        return httpx.Response(
            200,
            json={
                "model": model,
                "choices": [{"message": {"content": text}}],
                "usage": {"prompt_tokens": 1, "completion_tokens": 1},
            },
        )

    return httpx.AsyncClient(transport=httpx.MockTransport(handler))


def failing_client() -> httpx.AsyncClient:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(500, json={"error": "boom"})

    return httpx.AsyncClient(transport=httpx.MockTransport(handler))


@pytest.mark.anyio
async def test_dispatches_to_the_policys_primary_provider():
    router = LlmRouter(providers={"deepseek": DeepSeekProvider(client=ok_client("triaged"))})
    result = await router.complete("news_triage", CompletionRequest(prompt="breaking news"))
    assert result.text == "triaged"


@pytest.mark.anyio
async def test_falls_back_when_the_primary_provider_fails():
    router = LlmRouter(
        providers={
            "claude": AnthropicProvider(client=failing_client()),
            "openai": OpenAIProvider(client=ok_client("fallback worked")),
        }
    )
    result = await router.complete("critic", CompletionRequest(prompt="evaluate this signal"))
    assert result.text == "fallback worked"


@pytest.mark.anyio
async def test_raises_when_every_candidate_is_unavailable():
    router = LlmRouter(
        providers={
            "claude": AnthropicProvider(client=failing_client()),
            "openai": OpenAIProvider(client=failing_client()),
        }
    )
    with pytest.raises(AllProvidersUnavailableError):
        await router.complete("critic", CompletionRequest(prompt="evaluate this signal"))


@pytest.mark.anyio
async def test_skips_a_provider_that_lacks_a_required_capability():
    # deepseek has no vision capability; chart_vision requires it.
    router = LlmRouter(
        providers={
            "openai": DeepSeekProvider(client=ok_client("should not be reachable")),
            "claude": AnthropicProvider(client=ok_client("vision fallback")),
        }
    )
    # "openai" key deliberately holds a non-vision provider to prove the
    # requirement filter looks at real capabilities(), not just the name.
    result = await router.complete("chart_vision", CompletionRequest(prompt="describe this chart"))
    assert result.text == "vision fallback"


@pytest.mark.anyio
async def test_skips_an_unhealthy_provider_via_the_circuit_breaker():
    unhealthy = AnthropicProvider(client=failing_client())
    for _ in range(3):
        with pytest.raises(httpx.HTTPError):
            await unhealthy.complete(CompletionRequest(prompt="x"))
    assert unhealthy.health().healthy is False

    router = LlmRouter(
        providers={"claude": unhealthy, "openai": OpenAIProvider(client=ok_client("used instead"))}
    )
    result = await router.complete("critic", CompletionRequest(prompt="evaluate"))
    assert result.text == "used instead"


@pytest.mark.anyio
async def test_provider_spend_cap_routes_to_fallback_once_exhausted():
    router = LlmRouter(
        providers={
            "deepseek": DeepSeekProvider(client=ok_client("primary")),
            "kimi": KimiProvider(client=ok_client("fallback")),
        },
        provider_spend_caps={"deepseek": SpendCap(limit_usd=0.0)},
    )
    result = await router.complete("news_triage", CompletionRequest(prompt="x"))
    assert result.text == "fallback"


@pytest.mark.anyio
async def test_agent_spend_cap_blocks_every_provider_not_just_one():
    # Unlike a provider cap (which only takes that one provider out of the
    # fallback chain), an exhausted *agent* cap applies to every provider
    # that agent tries — there is no fallback around your own budget.
    router = LlmRouter(
        providers={
            "deepseek": DeepSeekProvider(client=ok_client("primary")),
            "kimi": KimiProvider(client=ok_client("fallback")),
        },
        agent_spend_caps={"news-agent": SpendCap(limit_usd=0.0)},
    )
    with pytest.raises(AllProvidersUnavailableError):
        await router.complete("news_triage", CompletionRequest(prompt="x"), agent="news-agent")

    # The same call with no agent attached is unaffected by that cap.
    result = await router.complete("news_triage", CompletionRequest(prompt="x"))
    assert result.text == "primary"


@pytest.mark.anyio
async def test_a_successful_call_charges_the_provider_and_agent_spend_caps():
    provider_cap = SpendCap(limit_usd=10.0)
    agent_cap = SpendCap(limit_usd=10.0)
    router = LlmRouter(
        providers={"deepseek": DeepSeekProvider(client=ok_client("ok"))},
        provider_spend_caps={"deepseek": provider_cap},
        agent_spend_caps={"news-agent": agent_cap},
    )
    await router.complete("news_triage", CompletionRequest(prompt="x"), agent="news-agent")
    assert provider_cap.spent_usd > 0
    assert agent_cap.spent_usd > 0
    assert provider_cap.spent_usd == agent_cap.spent_usd


@pytest.mark.anyio
async def test_cache_hit_short_circuits_the_provider_call_entirely():
    from agents_llm import CompletionResponse

    cache = SemanticCache()
    cache.store(
        "What is the market impact of US CPI on EURUSD?",
        CompletionResponse(text="cached answer", input_tokens=0, output_tokens=0, model="cache"),
    )
    # The only registered provider always fails — if the cache didn't
    # short-circuit the call, this would raise instead of returning.
    router = LlmRouter(
        providers={"deepseek": DeepSeekProvider(client=failing_client())}, cache=cache
    )

    result = await router.complete(
        "news_triage",
        CompletionRequest(prompt="What's the likely market impact of US CPI on EURUSD?"),
    )
    assert result.text == "cached answer"


@pytest.mark.anyio
async def test_every_dispatch_is_recorded_in_the_audit_log():
    router = LlmRouter(providers={"deepseek": DeepSeekProvider(client=ok_client("ok"))})
    await router.complete("news_triage", CompletionRequest(prompt="x"), signal_id="sig-1")

    assert len(router.audit_log) == 1
    entry = router.audit_log[0]
    assert entry.task_class == "news_triage"
    assert entry.provider == "deepseek"
    assert entry.signal_id == "sig-1"
    assert entry.cache_hit is False
    assert entry.cost_usd > 0
    assert len(entry.prompt_hash) == 16


@pytest.mark.anyio
async def test_a_cache_hit_is_also_recorded_in_the_audit_log():
    cache = SemanticCache()
    from agents_llm import CompletionResponse

    cache.store(
        "hello world",
        CompletionResponse(text="cached", input_tokens=0, output_tokens=0, model="cache"),
    )
    router = LlmRouter(
        providers={"deepseek": DeepSeekProvider(client=failing_client())}, cache=cache
    )

    await router.complete("news_triage", CompletionRequest(prompt="hello world"))

    assert router.audit_log[0].cache_hit is True
    assert router.audit_log[0].cost_usd == 0.0


@pytest.mark.anyio
async def test_unknown_task_class_raises_before_touching_any_provider():
    router = LlmRouter(providers={"deepseek": DeepSeekProvider(client=failing_client())})
    with pytest.raises(ValueError):
        await router.complete("not_a_real_task_class", CompletionRequest(prompt="x"))


@pytest.fixture
def anyio_backend():
    return "asyncio"
