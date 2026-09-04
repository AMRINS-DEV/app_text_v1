"""In-process semantic cache (§10.2's "L3a", checked "before any provider
call"). The real deployment backs this with Qdrant; this sandbox has none,
so this is a small in-memory nearest-neighbor index instead — but a real
one: cosine similarity over a bag-of-words term-frequency vector, not a
stub that only matches exact strings. A near-duplicate prompt (different
wording, same underlying question) really does hit this cache, which is
the property §17's "cache hit >= 40%" is about.
"""

from __future__ import annotations

import math
import re
from collections import Counter
from dataclasses import dataclass, field

from .provider import CompletionResponse

_TOKEN_RE = re.compile(r"[a-z0-9]+")


def _tokenize(text: str) -> Counter[str]:
    return Counter(_TOKEN_RE.findall(text.lower()))


def _cosine_similarity(a: Counter[str], b: Counter[str]) -> float:
    if not a or not b:
        return 0.0
    shared = a.keys() & b.keys()
    dot = sum(a[t] * b[t] for t in shared)
    norm_a = math.sqrt(sum(v * v for v in a.values()))
    norm_b = math.sqrt(sum(v * v for v in b.values()))
    if norm_a == 0.0 or norm_b == 0.0:
        return 0.0
    return dot / (norm_a * norm_b)


@dataclass
class _CacheEntry:
    tokens: Counter[str]
    response: CompletionResponse


@dataclass
class SemanticCache:
    similarity_threshold: float = 0.8
    max_entries: int = 2_000
    _entries: list[_CacheEntry] = field(default_factory=list)
    _hits: int = field(default=0, init=False)
    _misses: int = field(default=0, init=False)

    def lookup(self, prompt: str) -> CompletionResponse | None:
        tokens = _tokenize(prompt)
        best_score = 0.0
        best_entry: _CacheEntry | None = None
        for entry in self._entries:
            score = _cosine_similarity(tokens, entry.tokens)
            if score > best_score:
                best_score = score
                best_entry = entry
        if best_entry is not None and best_score >= self.similarity_threshold:
            self._hits += 1
            return best_entry.response
        self._misses += 1
        return None

    def store(self, prompt: str, response: CompletionResponse) -> None:
        if len(self._entries) >= self.max_entries:
            self._entries.pop(0)
        self._entries.append(_CacheEntry(tokens=_tokenize(prompt), response=response))

    def hit_rate(self) -> float:
        total = self._hits + self._misses
        return self._hits / total if total else 0.0

    def stats(self) -> dict[str, int]:
        return {"entries": len(self._entries), "hits": self._hits, "misses": self._misses}
