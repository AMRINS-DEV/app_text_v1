from __future__ import annotations

import numpy as np
from agents_pattern.geometry import detect_double_top_or_bottom, find_swing_points


def _ramp(start: float, end: float, n: int) -> np.ndarray:
    return np.linspace(start, end, n)


def _double_top_series() -> np.ndarray:
    # up to peak A (~110) -> down to neckline (~100) -> up to peak B (~110, same price) -> down.
    return np.concatenate(
        [
            _ramp(100, 110, 10),
            _ramp(110, 100, 10)[1:],
            _ramp(100, 110, 10)[1:],
            _ramp(110, 95, 10)[1:],
        ]
    )


def _double_bottom_series() -> np.ndarray:
    return np.concatenate(
        [
            _ramp(110, 100, 10),
            _ramp(100, 110, 10)[1:],
            _ramp(110, 100, 10)[1:],
            _ramp(100, 115, 10)[1:],
        ]
    )


def test_find_swing_points_recovers_the_obvious_turning_points():
    prices = _double_top_series()
    swings = find_swing_points(prices, prices, window=3)
    highs = [s.index for s in swings if s.kind == "high"]
    lows = [s.index for s in swings if s.kind == "low"]
    assert len(highs) >= 2
    assert len(lows) >= 1


def test_detects_a_double_top_and_computes_a_sane_measured_move_target():
    prices = _double_top_series()
    pattern = detect_double_top_or_bottom(prices, prices, window=3)
    assert pattern is not None
    assert pattern.kind == "double_top"
    # Target below the neckline by roughly the peak-to-neckline distance.
    assert pattern.target_price < pattern.neckline_price
    assert pattern.invalidation_price >= pattern.peak_price
    assert pattern.confidence > 0.9  # the two peaks are (by construction) nearly identical


def test_detects_a_double_bottom_and_computes_a_sane_measured_move_target():
    prices = _double_bottom_series()
    pattern = detect_double_top_or_bottom(prices, prices, window=3)
    assert pattern is not None
    assert pattern.kind == "double_bottom"
    assert pattern.target_price > pattern.neckline_price
    assert pattern.invalidation_price <= pattern.peak_price


def test_a_monotonic_trend_with_no_repeated_extrema_has_no_pattern():
    prices = _ramp(100, 150, 40)
    assert detect_double_top_or_bottom(prices, prices, window=3) is None


def test_two_peaks_too_far_apart_in_price_do_not_count_as_a_double_top():
    # Second peak is 10% higher than the first — well outside the default
    # 0.5% tolerance, so this should not be treated as a double top.
    series = np.concatenate(
        [
            _ramp(100, 110, 10),
            _ramp(110, 100, 10)[1:],
            _ramp(100, 130, 10)[1:],
            _ramp(130, 110, 10)[1:],
        ]
    )
    pattern = detect_double_top_or_bottom(series, series, window=3)
    assert pattern is None or pattern.kind != "double_top"
