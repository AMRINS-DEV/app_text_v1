"""Triple-barrier labeling (§8.2), transcribed verbatim from the design
doc's Python snippet — this one function is real because it's pure and
exactly matches the doc's own worked example. Meta-labeling (the second
stage predicting P(primary is correct)) is Phase 3 scope: it needs a
trained primary model to label against.
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from enum import IntEnum


class Barrier(IntEnum):
    LOSS = -1
    TIMEOUT = 0
    WIN = 1


@dataclass(frozen=True)
class TripleBarrierResult:
    label: Barrier
    bars_to_resolution: int


def triple_barrier(
    prices: Sequence[float],
    t0: int,
    atr: float,
    tp_mult: float = 2.2,
    sl_mult: float = 1.5,
    max_bars: int = 40,
) -> TripleBarrierResult:
    """§8.2: label = +1 if TP hit first, -1 if SL hit first, 0 if time-out."""
    entry = prices[t0]
    upper = entry + tp_mult * atr
    lower = entry - sl_mult * atr
    path = prices[t0 + 1 : t0 + 1 + max_bars]
    for i, p in enumerate(path):
        if p >= upper:
            return TripleBarrierResult(Barrier.WIN, i)
        if p <= lower:
            return TripleBarrierResult(Barrier.LOSS, i)
    return TripleBarrierResult(Barrier.TIMEOUT, len(path))
