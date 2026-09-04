"""MCP tool servers (§3.2, §7): "same tools usable from Claude Desktop for
manual research." Two of the four planned servers are real here —
market-data and journal; `timeseries` (QuestDB) and `graph` (FalkorDB
queries per §7.2) need infrastructure this sandbox doesn't have and are
Phase 6+ scope, same as `crates/storage`.

Deliberately thin: the FastMCP `@mcp.tool()` wrappers below do no work of
their own beyond calling into `synthetic_data.py`'s plain functions and
shaping the result as a dict — the generation logic itself is unit-tested
directly, without needing a real MCP client/transport in the loop.
"""

from __future__ import annotations

from dataclasses import asdict

from mcp.server.fastmcp import FastMCP

from .synthetic_data import generate_bars, generate_trade_history

mcp = FastMCP("tradeos-market-and-journal")


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


def main() -> None:
    mcp.run()


if __name__ == "__main__":
    main()
