"""Routing policy table, verbatim from design doc §10.2, plus the real
router: capability matching, cost/latency budgets, circuit breaking
(delegated to each provider's own `health()`), spend caps, semantic
caching, and a full audit trail.
"""

from __future__ import annotations

import asyncio
import hashlib
import time
from dataclasses import dataclass, field
from typing import cast

import httpx

from .provider import CompletionRequest, CompletionResponse, LlmProvider
from .semantic_cache import SemanticCache

ROUTING_POLICY: dict[str, dict[str, object]] = {
    "news_triage": {"primary": "deepseek", "fallback": ["kimi", "openai"], "max_latency_s": 3},
    "news_deep": {"primary": "claude", "fallback": ["openai"], "max_latency_s": 20},
    "chart_vision": {"primary": "openai", "fallback": ["claude"], "requires": ["vision"]},
    "critic": {"primary": "claude", "fallback": ["openai"], "max_latency_s": 10},
    "session_review": {"primary": "kimi", "fallback": ["claude"], "requires": ["ctx>200k"]},
    "pattern_narrative": {"primary": "deepseek", "fallback": ["kimi"]},
}


@dataclass
class SpendCap:
    """A rolling spend cap (§10.2: "a monthly spend cap enforced per
    provider and per agent"). `period` labeling (which month) is the
    caller's responsibility — this just tracks spent-vs-cap for whatever
    bucket key it's registered under."""

    limit_usd: float
    spent_usd: float = 0.0

    def remaining_usd(self) -> float:
        return max(self.limit_usd - self.spent_usd, 0.0)

    def can_afford(self, estimated_cost_usd: float) -> bool:
        return self.spent_usd + estimated_cost_usd <= self.limit_usd

    def charge(self, cost_usd: float) -> None:
        self.spent_usd += cost_usd


@dataclass
class AuditEntry:
    """§10.2: "every call logged with prompt hash, provider, model, tokens,
    cost, latency, and the resulting signal ID." """

    task_class: str
    provider: str
    model: str
    prompt_hash: str
    input_tokens: int
    output_tokens: int
    cost_usd: float
    latency_s: float
    cache_hit: bool
    agent: str | None = None
    signal_id: str | None = None


class AllProvidersUnavailableError(Exception):
    """Every candidate for a task class was unhealthy, over budget, or
    itself failed/timed out."""


def _prompt_hash(prompt: str) -> str:
    return hashlib.sha256(prompt.encode("utf-8")).hexdigest()[:16]


def _requirement_satisfied(requirement: str, provider: LlmProvider) -> bool:
    caps = provider.capabilities()
    if requirement == "vision":
        return caps.vision
    if requirement.startswith("ctx>"):
        return caps.ctx_len > int(requirement[len("ctx>") :])
    return True


@dataclass
class LlmRouter:
    providers: dict[str, LlmProvider] = field(default_factory=dict)
    policy: dict[str, dict[str, object]] = field(default_factory=lambda: dict(ROUTING_POLICY))
    cache: SemanticCache | None = None
    provider_spend_caps: dict[str, SpendCap] = field(default_factory=dict)
    agent_spend_caps: dict[str, SpendCap] = field(default_factory=dict)
    audit_log: list[AuditEntry] = field(default_factory=list)

    def route(self, task_class: str) -> str:
        """The primary provider name for a task class, ignoring health/
        budget/capability filtering — kept for callers that only want the
        static policy answer, not a live dispatch."""
        try:
            return str(self.policy[task_class]["primary"])
        except KeyError as e:
            raise ValueError(f"no routing policy for task class {task_class!r}") from e

    def _candidates(self, task_class: str) -> list[str]:
        cfg = self.policy[task_class]
        fallback = cast("list[str]", cfg.get("fallback", []))
        names = [cast(str, cfg["primary"]), *fallback]
        requirements = cast("list[str]", cfg.get("requires", []))
        candidates = []
        for name in names:
            provider = self.providers.get(name)
            if provider is None:
                continue
            if all(_requirement_satisfied(r, provider) for r in requirements):
                candidates.append(name)
        return candidates

    def _estimate_cost_usd(self, provider: LlmProvider, req: CompletionRequest) -> float:
        cost = provider.cost_per_mtok()
        # Upper-bound estimate before the call: len(prompt)/4 as a rough
        # input-token count, plus the requested max_tokens as the
        # worst-case output — real cost (computed from actual usage) is
        # what's charged after the call completes.
        estimated_input_tokens = max(len(req.prompt) // 4, 1)
        return (estimated_input_tokens / 1_000_000) * cost.input_per_mtok_usd + (
            req.max_tokens / 1_000_000
        ) * cost.output_per_mtok_usd

    def _actual_cost_usd(self, provider: LlmProvider, response: CompletionResponse) -> float:
        cost = provider.cost_per_mtok()
        return (response.input_tokens / 1_000_000) * cost.input_per_mtok_usd + (
            response.output_tokens / 1_000_000
        ) * cost.output_per_mtok_usd

    def _within_budget(self, name: str, agent: str | None, estimated_cost_usd: float) -> bool:
        provider_cap = self.provider_spend_caps.get(name)
        if provider_cap is not None and not provider_cap.can_afford(estimated_cost_usd):
            return False
        if agent is not None:
            agent_cap = self.agent_spend_caps.get(agent)
            if agent_cap is not None and not agent_cap.can_afford(estimated_cost_usd):
                return False
        return True

    def _charge(self, name: str, agent: str | None, cost_usd: float) -> None:
        provider_cap = self.provider_spend_caps.get(name)
        if provider_cap is not None:
            provider_cap.charge(cost_usd)
        if agent is not None:
            agent_cap = self.agent_spend_caps.get(agent)
            if agent_cap is not None:
                agent_cap.charge(cost_usd)

    async def complete(
        self,
        task_class: str,
        req: CompletionRequest,
        *,
        agent: str | None = None,
        signal_id: str | None = None,
    ) -> CompletionResponse:
        if task_class not in self.policy:
            raise ValueError(f"no routing policy for task class {task_class!r}")

        if self.cache is not None:
            cached = self.cache.lookup(req.prompt)
            if cached is not None:
                self.audit_log.append(
                    AuditEntry(
                        task_class=task_class,
                        provider="cache",
                        model=cached.model,
                        prompt_hash=_prompt_hash(req.prompt),
                        input_tokens=0,
                        output_tokens=0,
                        cost_usd=0.0,
                        latency_s=0.0,
                        cache_hit=True,
                        agent=agent,
                        signal_id=signal_id,
                    )
                )
                return cached

        cfg = self.policy[task_class]
        max_latency_s = cast("float | int | None", cfg.get("max_latency_s"))
        last_error: Exception | None = None

        for name in self._candidates(task_class):
            provider = self.providers[name]
            if not provider.health().healthy:
                continue
            estimated_cost = self._estimate_cost_usd(provider, req)
            if not self._within_budget(name, agent, estimated_cost):
                continue

            started = time.monotonic()
            try:
                if max_latency_s is not None:
                    response = await asyncio.wait_for(
                        provider.complete(req), timeout=float(max_latency_s)
                    )
                else:
                    response = await provider.complete(req)
            except (httpx.HTTPError, TimeoutError) as exc:
                last_error = exc
                continue
            latency_s = time.monotonic() - started

            cost_usd = self._actual_cost_usd(provider, response)
            self._charge(name, agent, cost_usd)
            self.audit_log.append(
                AuditEntry(
                    task_class=task_class,
                    provider=name,
                    model=response.model,
                    prompt_hash=_prompt_hash(req.prompt),
                    input_tokens=response.input_tokens,
                    output_tokens=response.output_tokens,
                    cost_usd=cost_usd,
                    latency_s=latency_s,
                    cache_hit=False,
                    agent=agent,
                    signal_id=signal_id,
                )
            )
            if self.cache is not None:
                self.cache.store(req.prompt, response)
            return response

        raise AllProvidersUnavailableError(
            f"no healthy, in-budget, capable provider for task class {task_class!r}"
        ) from last_error
