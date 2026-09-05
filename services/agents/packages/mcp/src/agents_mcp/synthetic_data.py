"""Deterministic synthetic market-data and journal generators backing the
MCP tools in `server.py` — this sandbox has no QuestDB/Postgres to read
real bars or trade history from, the same split as the gateway's own
`historical-bars.ts`/`trade-history.ts` (independent Python
implementations, not a cross-language port — each side generates its own
synthetic data with its own seed).
"""

from __future__ import annotations

import random
from dataclasses import dataclass

# A fixed reference instant, not the wall clock — a generator whose default
# end time is "now" silently stops being deterministic between two calls a
# moment apart (a real bug caught this exact way in the gateway's own
# trade-history generator; fixed there the same way, fixed here from the start).
_FIXED_REFERENCE_MS = 1_767_225_600_000  # 2026-01-01T00:00:00Z


@dataclass(frozen=True)
class Bar:
    symbol: str
    timeframe: str
    open_time_ms: int
    open: float
    high: float
    low: float
    close: float
    volume: int


_TIMEFRAME_MS = {"1m": 60_000, "5m": 300_000, "1h": 3_600_000}


def generate_bars(
    symbol: str,
    timeframe: str,
    count: int,
    *,
    end_time_ms: int = _FIXED_REFERENCE_MS,
) -> list[Bar]:
    """`count` bars ending at `end_time_ms`, keyed on `(symbol, timeframe,
    open_time_ms)` so the same bar always resolves to the same OHLCV no
    matter when it's requested."""
    period_ms = _TIMEFRAME_MS.get(timeframe)
    if period_ms is None:
        msg = f"unknown timeframe: {timeframe!r}"
        raise ValueError(msg)
    if count <= 0:
        msg = "count must be positive"
        raise ValueError(msg)

    last_open = (end_time_ms // period_ms) * period_ms
    bars: list[Bar] = []
    for i in range(count):
        open_time = last_open - (count - 1 - i) * period_ms
        rng = random.Random(f"{symbol}:{timeframe}:{open_time}")
        base = 1.0 + rng.random() * 2.0
        open_price = base
        close_price = base + (rng.random() - 0.5) * 0.02 * base
        high_price = max(open_price, close_price) + rng.random() * 0.01 * base
        low_price = min(open_price, close_price) - rng.random() * 0.01 * base
        bars.append(
            Bar(
                symbol=symbol,
                timeframe=timeframe,
                open_time_ms=open_time,
                open=open_price,
                high=high_price,
                low=low_price,
                close=close_price,
                volume=rng.randint(0, 1000),
            )
        )
    return bars


@dataclass(frozen=True)
class ClosedTrade:
    closed_at_ms: int
    symbol: str
    pnl: float


def generate_trade_history(
    count: int = 90,
    *,
    symbol: str = "EURUSD",
    seed: int = 1,
    end_time_ms: int = _FIXED_REFERENCE_MS,
    span_ms: int = 30 * 24 * 60 * 60 * 1000,
) -> list[ClosedTrade]:
    """A deterministic synthetic closed-trade history — same "real
    computation, synthetic input" split as the gateway's `/api/stats/
    overview` and Phase 3's ML training data: there is no ingested trade
    ledger in this environment to read a real one from."""
    if count <= 0:
        msg = "count must be positive"
        raise ValueError(msg)
    rng = random.Random(seed)
    trades: list[ClosedTrade] = []
    for i in range(count):
        closed_at = end_time_ms - span_ms + int((i / count) * span_ms)
        pnl = rng.gauss(35, 220)
        trades.append(ClosedTrade(closed_at_ms=closed_at, symbol=symbol, pnl=pnl))
    return trades
