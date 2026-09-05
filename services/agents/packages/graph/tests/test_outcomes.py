import pytest
from agents_graph import (
    Verdict,
    resolve_fixed_horizon_move,
    resolve_pattern_outcome,
)


def test_long_pattern_confirmed_when_target_hit_before_invalidation():
    resolution = resolve_pattern_outcome(
        direction="Long",
        entry_price=100.0,
        target_price=104.0,
        invalidation_price=98.0,
        subsequent_highs=[101.0, 102.5, 104.5],
        subsequent_lows=[99.5, 100.5, 103.0],
        pip_size=0.1,
        atr=2.0,
    )
    assert resolution.verdict == Verdict.CONFIRMED
    assert resolution.bars_to_resolution == 2
    assert resolution.r_multiple == 2.0  # (104-100)/(100-98) = 2R
    assert resolution.move_pips == 40.0  # 4.0 / 0.1
    assert resolution.move_atr == 2.0  # 4.0 / 2.0
    assert resolution.mfe == 4.5  # best high (104.5) - entry


def test_short_pattern_failed_when_invalidation_hit_before_target():
    resolution = resolve_pattern_outcome(
        direction="Short",
        entry_price=100.0,
        target_price=96.0,
        invalidation_price=102.0,
        subsequent_highs=[100.5, 101.0, 103.0],
        subsequent_lows=[99.0, 98.5, 98.0],
        pip_size=0.1,
        atr=2.0,
    )
    assert resolution.verdict == Verdict.FAILED
    assert resolution.bars_to_resolution == 2
    assert resolution.r_multiple == -1.0  # invalidation hit -> -1R by definition
    assert resolution.move_pips == -20.0


def test_pattern_times_out_when_neither_barrier_is_hit():
    resolution = resolve_pattern_outcome(
        direction="Long",
        entry_price=100.0,
        target_price=110.0,
        invalidation_price=90.0,
        subsequent_highs=[101.0, 102.0],
        subsequent_lows=[99.0, 100.0],
        pip_size=0.1,
        atr=2.0,
    )
    assert resolution.verdict == Verdict.TIMEOUT
    assert resolution.bars_to_resolution is None
    assert resolution.r_multiple == 0.0
    assert resolution.mfe == 2.0


def test_a_bar_touching_both_barriers_resolves_toward_invalidation():
    resolution = resolve_pattern_outcome(
        direction="Long",
        entry_price=100.0,
        target_price=104.0,
        invalidation_price=98.0,
        subsequent_highs=[105.0],  # would hit target...
        subsequent_lows=[97.0],  # ...but also hits invalidation in the same bar
        pip_size=0.1,
        atr=2.0,
    )
    assert resolution.verdict == Verdict.FAILED


def test_invalid_direction_is_rejected():
    with pytest.raises(ValueError, match="direction"):
        resolve_pattern_outcome(
            direction="sideways",
            entry_price=100.0,
            target_price=104.0,
            invalidation_price=98.0,
            subsequent_highs=[101.0],
            subsequent_lows=[99.0],
            pip_size=0.1,
            atr=2.0,
        )


def test_fixed_horizon_move_detects_a_direction_hit():
    move = resolve_fixed_horizon_move(
        price_at_event=1.1000,
        price_at_horizon=1.1050,
        expected_direction="Long",
        pip_size=0.0001,
        atr=0.0020,
    )
    assert move.direction == "Long"
    assert move.direction_hit is True
    assert move.move_pips == pytest.approx(50.0)
    assert move.move_atr == pytest.approx(2.5)


def test_fixed_horizon_move_detects_a_direction_miss():
    move = resolve_fixed_horizon_move(
        price_at_event=1.1000,
        price_at_horizon=1.0950,
        expected_direction="Long",
        pip_size=0.0001,
        atr=0.0020,
    )
    assert move.direction == "Short"
    assert move.direction_hit is False


def test_fixed_horizon_move_with_no_movement_is_flat():
    move = resolve_fixed_horizon_move(
        price_at_event=1.1000,
        price_at_horizon=1.1000,
        expected_direction="Flat",
        pip_size=0.0001,
        atr=0.0020,
    )
    assert move.direction == "Flat"
    assert move.direction_hit is True
