"""MCP tool servers (§3.2, §7). Four servers planned: market-data,
timeseries, graph (FalkorDB queries from §7.2), journal. Phase 5/6 scope."""

SERVER_NAMES = ("market", "timeseries", "graph", "journal")
"""Names only for Phase 0 — each becomes a real MCP server module in Phase 5/6."""
