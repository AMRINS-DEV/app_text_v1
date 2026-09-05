"""MCP tool servers (§3.2, §7): "same tools usable from Claude Desktop for
manual research." Three of the four planned servers are real here —
market-data, journal, and graph (§7.2's conditional-reliability/news-
impact/confluence queries, against the in-memory `KnowledgeGraph` that
stands in for FalkorDB — see `agents_graph.store`'s doc comment for why).
`timeseries` (QuestDB) needs infrastructure this sandbox doesn't have and
remains Phase 6+ scope, same as `crates/storage`.

Deliberately thin: the FastMCP `@mcp.tool()` wrappers below do no work of
their own beyond calling into `synthetic_data.py`/`agents_graph`'s plain
functions and shaping the result as a dict — the generation/query logic
itself is unit-tested directly, without needing a real MCP client/
transport in the loop.
"""

from __future__ import annotations

from dataclasses import asdict

from agents_graph import (
    build_backfilled_graph,
    conditional_reliability,
    confluence_discovery,
    news_impact_persistence,
)
from mcp.server.fastmcp import FastMCP

from .synthetic_data import generate_bars, generate_trade_history

mcp = FastMCP("tradeos-market-journal-and-graph")

# Built once at import time from deterministic synthetic history (Prompt
# 7's "backfill script for historical news and patterns") — a real
# deployment would load this from a live FalkorDB instead of rebuilding it
# per process start.
_GRAPH = build_backfilled_graph()


@mcp.tool()
def get_bars(symbol: str, timeframe: str, count: int = 100) -> list[dict[str, object]]:
    """Returns `count` recent OHLCV bars for `symbol` at `timeframe`
    ("1m", "5m", or "1h"). Synthetic data — see this module's doc comment."""
    return [asdict(bar) for bar in generate_bars(symbol, timeframe, count)]


@mcp.tool()
def get_trade_history(count: int = 90, symbol: str = "EURUSD") -> list[dict[str, object]]:
    """Returns `count` recent closed trades for `symbol`. Synthetic data —
    see this module's doc comment."""
    return [asdict(trade) for trade in generate_trade_history(count, symbol=symbol)]


@mcp.tool()
def get_pattern_reliability(
    pattern_name: str, symbol: str, regime_label: str, since_ts: int = 0
) -> dict[str, object]:
    """§7.2 query #1: does `pattern_name` work on `symbol` in
    `regime_label`? Always check the returned `n` against the §8.7
    sample-size gate (>=30) before trusting `hit_rate`/`avg_r`."""
    result = conditional_reliability(
        _GRAPH,
        pattern_name=pattern_name,
        symbol=symbol,
        regime_label=regime_label,
        since_ts=since_ts,
    )
    return asdict(result)


@mcp.tool()
def get_news_impact_stability(
    event_type_name: str, symbol: str, horizon_min: int, expected_direction: str
) -> list[dict[str, object]]:
    """§7.2 query #2: is `event_type_name`'s effect on `symbol` stable or
    decaying over time, bucketed by quarter?"""
    periods = news_impact_persistence(
        _GRAPH,
        event_type_name=event_type_name,
        symbol=symbol,
        horizon_min=horizon_min,
        expected_direction=expected_direction,
    )
    return [asdict(period) for period in periods]


@mcp.tool()
def get_confluence(
    max_lag_bars: int = 3, min_n: int = 40, limit: int = 20
) -> list[dict[str, object]]:
    """§7.2 query #3: which pattern-pair combinations beat their parts?"""
    results = confluence_discovery(_GRAPH, max_lag_bars=max_lag_bars, min_n=min_n, limit=limit)
    return [asdict(result) for result in results]


def main() -> None:
    mcp.run()


if __name__ == "__main__":
    main()
