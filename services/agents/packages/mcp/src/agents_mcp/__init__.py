"""MCP tool servers (§3.2, §7). `market`, `journal`, and `graph` are real
(see `server.py`, `synthetic_data.py`, and `agents_graph` for the graph
tools' backing engine); `timeseries` needs QuestDB and remains Phase 6+
scope.
"""

from .server import (
    get_bars,
    get_confluence,
    get_news_impact_stability,
    get_pattern_reliability,
    get_trade_history,
    mcp,
)
from .synthetic_data import Bar, ClosedTrade, generate_bars, generate_trade_history

SERVER_NAMES = ("market", "journal", "graph")
"""`timeseries` is still names-only — needs QuestDB, Phase 6+ scope."""

__all__ = [
    "SERVER_NAMES",
    "mcp",
    "get_bars",
    "get_trade_history",
    "get_pattern_reliability",
    "get_news_impact_stability",
    "get_confluence",
    "Bar",
    "ClosedTrade",
    "generate_bars",
    "generate_trade_history",
]
