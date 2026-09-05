import numpy as np
import pytest
from agents_models.cross_validation import walk_forward_windows
from agents_validation import generate_labeled_series
from agents_validation.backtest import SAMPLE_SIZE_GATE, ExpectancyGateConfig
from agents_validation.walk_forward import run_walk_forward

R_TARGET = 2.2 / 1.5


def test_run_walk_forward_produces_one_fold_per_window():
    series = generate_labeled_series(n_bars=4000, seed=3)
    expected_windows = walk_forward_windows(
        len(series.labels), train_periods=1500, validate_periods=300, test_periods=200
    )

    result = run_walk_forward(
        series, train_periods=1500, validate_periods=300, test_periods=200, r_target=R_TARGET
    )

    assert len(result.folds) == len(expected_windows)
    assert len(expected_windows) > 1  # otherwise this test proves nothing about "walking forward"


def test_every_folds_test_range_is_disjoint_from_every_other_folds():
    series = generate_labeled_series(n_bars=4000, seed=3)
    result = run_walk_forward(
        series, train_periods=1500, validate_periods=300, test_periods=200, r_target=R_TARGET
    )
    test_ranges = [(fold.window.test_start, fold.window.test_end) for fold in result.folds]
    for i, (start_a, end_a) in enumerate(test_ranges):
        for start_b, end_b in test_ranges[i + 1 :]:
            assert end_a <= start_b or end_b <= start_a


def test_calibrated_probabilities_and_y_true_are_only_from_the_test_range():
    series = generate_labeled_series(n_bars=4000, seed=3)
    result = run_walk_forward(
        series, train_periods=1500, validate_periods=300, test_periods=200, r_target=R_TARGET
    )
    for fold in result.folds:
        expected_len = fold.window.test_end - fold.window.test_start
        assert len(fold.calibrated_probabilities) == expected_len
        assert len(fold.y_true) == expected_len
        assert np.all((fold.calibrated_probabilities > 0.0) & (fold.calibrated_probabilities < 1.0))


def test_all_trades_pools_trades_across_every_fold():
    series = generate_labeled_series(n_bars=4000, seed=3)
    result = run_walk_forward(
        series, train_periods=1500, validate_periods=300, test_periods=200, r_target=R_TARGET
    )
    assert len(result.all_trades) == sum(fold.backtest.n_trades for fold in result.folds)


def test_expectancy_is_the_mean_r_multiple_across_all_pooled_trades():
    series = generate_labeled_series(n_bars=4000, seed=3)
    result = run_walk_forward(
        series, train_periods=1500, validate_periods=300, test_periods=200, r_target=R_TARGET
    )
    if result.all_trades:
        expected = np.mean([t.r_multiple for t in result.all_trades])
        assert result.expectancy == pytest.approx(expected)
    else:
        assert result.expectancy == 0.0


def test_a_series_too_short_for_the_requested_windows_raises():
    series = generate_labeled_series(n_bars=200, seed=1)
    with pytest.raises(ValueError, match="too short"):
        run_walk_forward(
            series, train_periods=1500, validate_periods=300, test_periods=200, r_target=R_TARGET
        )


def test_a_permissive_gate_produces_more_trades_than_a_strict_one():
    series = generate_labeled_series(n_bars=4000, seed=3)
    strict = run_walk_forward(
        series,
        train_periods=1500,
        validate_periods=300,
        test_periods=200,
        r_target=R_TARGET,
        gate_config=ExpectancyGateConfig(r_target=R_TARGET, p_min=0.9, theta=1.0),
    )
    permissive = run_walk_forward(
        series,
        train_periods=1500,
        validate_periods=300,
        test_periods=200,
        r_target=R_TARGET,
        gate_config=ExpectancyGateConfig(r_target=R_TARGET, p_min=0.5, theta=-1.0),
    )
    assert len(permissive.all_trades) >= len(strict.all_trades)


def test_windows_shorter_than_the_sample_size_gate_produce_no_trades_but_no_crash():
    series = generate_labeled_series(n_bars=2000, seed=1)
    result = run_walk_forward(
        series,
        train_periods=1000,
        validate_periods=200,
        test_periods=SAMPLE_SIZE_GATE - 5,
        r_target=R_TARGET,
    )
    assert result.all_trades == []
    assert result.expectancy == 0.0
