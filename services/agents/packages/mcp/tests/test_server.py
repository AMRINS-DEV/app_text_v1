import pytest
from agents_mcp import mcp
from mcp.server.fastmcp.exceptions import ToolError


@pytest.mark.anyio
async def test_every_real_tool_is_registered():
    tools = await mcp.list_tools()
    names = {tool.name for tool in tools}
    assert names == {
        "get_bars",
        "get_trade_history",
        "get_pattern_reliability",
        "get_news_impact_stability",
        "get_confluence",
    }


@pytest.mark.anyio
async def test_get_bars_tool_returns_the_requested_count():
    _, result = await mcp.call_tool("get_bars", {"symbol": "EURUSD", "timeframe": "1m", "count": 5})
    assert len(result["result"]) == 5
    assert result["result"][0]["symbol"] == "EURUSD"


@pytest.mark.anyio
async def test_get_trade_history_tool_returns_the_requested_count():
    _, result = await mcp.call_tool("get_trade_history", {"count": 7, "symbol": "GBPUSD"})
    assert len(result["result"]) == 7
    assert result["result"][0]["symbol"] == "GBPUSD"


@pytest.mark.anyio
async def test_get_pattern_reliability_tool_returns_the_expected_shape():
    _, payload = await mcp.call_tool(
        "get_pattern_reliability",
        {"pattern_name": "double_top", "symbol": "EURUSD", "regime_label": "Trending"},
    )
    assert set(payload) == {"n", "hit_rate", "avg_r", "median_r"}
    assert isinstance(payload["n"], int)


@pytest.mark.anyio
async def test_get_pattern_reliability_tool_with_an_unknown_pattern_reports_n_zero():
    _, payload = await mcp.call_tool(
        "get_pattern_reliability",
        {"pattern_name": "not-a-real-pattern", "symbol": "EURUSD", "regime_label": "Trending"},
    )
    assert payload["n"] == 0


@pytest.mark.anyio
async def test_get_news_impact_stability_tool_returns_a_list_of_periods():
    _, result = await mcp.call_tool(
        "get_news_impact_stability",
        {
            "event_type_name": "rate_decision",
            "symbol": "EURUSD",
            "horizon_min": 15,
            "expected_direction": "Long",
        },
    )
    periods = result["result"]
    assert isinstance(periods, list)
    for period in periods:
        assert set(period) == {"quarter", "n", "avg_impact", "direction_hit_rate"}


@pytest.mark.anyio
async def test_get_confluence_tool_returns_a_list_within_the_requested_limit():
    _, result = await mcp.call_tool("get_confluence", {"min_n": 1, "limit": 5})
    combos = result["result"]
    assert isinstance(combos, list)
    assert len(combos) <= 5
    for combo in combos:
        assert set(combo) == {"pattern_a_name", "pattern_b_name", "n", "combo_r"}


@pytest.mark.anyio
async def test_get_bars_tool_rejects_an_unknown_timeframe():
    # Calling `call_tool` directly (as these tests do, with no real
    # transport in between) re-raises the tool's own exception rather than
    # returning an error-flagged result — the wire-protocol-level error
    # framing is FastMCP's own tested behavior, not this repo's.
    with pytest.raises(ToolError, match="timeframe"):
        await mcp.call_tool("get_bars", {"symbol": "EURUSD", "timeframe": "3m", "count": 5})


@pytest.fixture
def anyio_backend():
    return "asyncio"
