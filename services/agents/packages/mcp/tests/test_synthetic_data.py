import pytest
from agents_mcp.synthetic_data import generate_bars, generate_trade_history


def test_generate_bars_is_deterministic():
    a = generate_bars("EURUSD", "1m", 20)
    b = generate_bars("EURUSD", "1m", 20)
    assert a == b


def test_generate_bars_produces_the_requested_count_in_ascending_time_order():
    bars = generate_bars("EURUSD", "5m", 15)
    assert len(bars) == 15
    times = [b.open_time_ms for b in bars]
    assert times == sorted(times)
    assert len(set(times)) == len(times)


def test_generate_bars_high_low_bracket_open_close():
    bars = generate_bars("XAUUSD", "1h", 30)
    for bar in bars:
        assert bar.high >= max(bar.open, bar.close)
        assert bar.low <= min(bar.open, bar.close)


def test_generate_bars_rejects_an_unknown_timeframe():
    with pytest.raises(ValueError, match="timeframe"):
        generate_bars("EURUSD", "3m", 10)


def test_generate_bars_rejects_a_non_positive_count():
    with pytest.raises(ValueError, match="count"):
        generate_bars("EURUSD", "1m", 0)


def test_different_symbols_produce_different_bars():
    a = generate_bars("EURUSD", "1m", 5)
    b = generate_bars("GBPUSD", "1m", 5)
    assert a != b


def test_generate_trade_history_is_deterministic():
    a = generate_trade_history(50)
    b = generate_trade_history(50)
    assert a == b


def test_generate_trade_history_produces_the_requested_count_in_ascending_time_order():
    trades = generate_trade_history(40)
    assert len(trades) == 40
    times = [t.closed_at_ms for t in trades]
    assert times == sorted(times)


def test_generate_trade_history_rejects_a_non_positive_count():
    with pytest.raises(ValueError, match="count"):
        generate_trade_history(0)


def test_different_seeds_produce_different_trade_histories():
    a = generate_trade_history(20, seed=1)
    b = generate_trade_history(20, seed=2)
    assert a != b
