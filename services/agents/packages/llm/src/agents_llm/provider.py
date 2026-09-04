"""The unified provider interface every LLM sits behind (§10.2, §3.2's
"Multi-provider LLM vs consistency: per-task routing with per-model
calibration; model version is part of the calibration key")."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Protocol


@dataclass(frozen=True)
class Caps:
    vision: bool = False
    tools: bool = False
    ctx_len: int = 0
    json_mode: bool = False


@dataclass(frozen=True)
class Cost:
    input_per_mtok_usd: float
    output_per_mtok_usd: float


@dataclass(frozen=True)
class Health:
    healthy: bool
    consecutive_failures: int = 0


@dataclass(frozen=True)
class CompletionRequest:
    prompt: str
    max_tokens: int = 1024
    json_mode: bool = False


@dataclass(frozen=True)
class CompletionResponse:
    text: str
    input_tokens: int
    output_tokens: int
    model: str


class LlmProvider(Protocol):
    async def complete(self, req: CompletionRequest) -> CompletionResponse: ...
    def capabilities(self) -> Caps: ...
    def cost_per_mtok(self) -> Cost: ...
    def health(self) -> Health: ...
