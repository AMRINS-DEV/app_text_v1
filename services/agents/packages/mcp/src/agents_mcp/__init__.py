"""MCP tool servers (§3.2, §7). `market` and `journal` are real (see
`server.py`, `synthetic_data.py`); `timeseries` and `graph` need
QuestDB/FalkorDB and remain Phase 6+ scope.
"""

from .server import get_bars, get_trade_history, mcp
from .synthetic_data import Bar, ClosedTrade, generate_bars, generate_trade_history

SERVER_NAMES = ("market", "journal")
"""`timeseries` and `graph` are still names-only — Phase 6+ scope."""

__all__ = [
    "SERVER_NAMES",
    "mcp",
    "get_bars",
    "get_trade_history",
    "Bar",
    "ClosedTrade",
    "generate_bars",
    "generate_trade_history",
]
