import pytest
from agents_mcp import mcp
from mcp.server.fastmcp.exceptions import ToolError


@pytest.mark.anyio
async def test_both_real_tools_are_registered():
    tools = await mcp.list_tools()
    names = {tool.name for tool in tools}
    assert names == {"get_bars", "get_trade_history"}


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
