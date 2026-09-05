import numpy as np
import pytest
from agents_models.calibration import IsotonicCalibrator
from agents_models.labeling import Barrier
from agents_models.training import TrainConfig, raw_probability, train_gbdt
from agents_validation import generate_labeled_series, y_true_from_labels
from agents_validation.backtest import (
    SAMPLE_SIZE_GATE,
    ExpectancyGateConfig,
    expected_r,
    passes_expectancy_gate,
    run_backtest,
)

R_TARGET = 2.2 / 1.5  # matches the dataset's own default tp_mult/sl_mult ratio


def test_expected_r_matches_the_8_5_formula_by_hand():
    config = ExpectancyGateConfig(r_target=2.0, cost_r=0.05)
    # p=0.6: 0.6*2.0 - 0.4*1.0 - 0.05 = 1.2 - 0.4 - 0.05 = 0.75
    assert expected_r(0.6, config) == pytest.approx(0.75)


def test_gate_rejects_a_probability_below_p_min():
    config = ExpectancyGateConfig(r_target=2.0, p_min=0.55)
    assert not passes_expectancy_gate(0.5, config)


def test_gate_rejects_expectancy_below_theta_even_with_high_probability():
    # p=0.56 clears p_min but E[R] = 0.56*1.1 - 0.44 - 0.05 = 0.126, below theta=0.15.
    config = ExpectancyGateConfig(r_target=1.1, p_min=0.55, theta=0.15, cost_r=0.05)
    assert not passes_expectancy_gate(0.56, config)


def test_gate_accepts_a_genuinely_favorable_setup():
    config = ExpectancyGateConfig(r_target=2.2, p_min=0.55, theta=0.15, cost_r=0.05)
    assert passes_expectancy_gate(0.7, config)


def test_gate_enforces_the_cost_ceiling_regardless_of_probability():
    # cost_r=0.3 > 0.10 * r_target=2.0 -> vetoed no matter how high p is.
    config = ExpectancyGateConfig(r_target=2.0, cost_r=0.3)
    assert not passes_expectancy_gate(0.99, config)


def test_run_backtest_never_trades_during_the_warm_up():
    n = SAMPLE_SIZE_GATE + 10
    probabilities = np.full(n, 0.99)  # certain win, would clear any gate
    labels = [Barrier.WIN] * n
    config = ExpectancyGateConfig(r_target=2.0, p_min=0.5, theta=0.0, cost_r=0.0)

    result = run_backtest(probabilities, labels, config)

    assert all(trade.entry_index >= SAMPLE_SIZE_GATE for trade in result.trades)
    assert result.n_candidates == n - SAMPLE_SIZE_GATE


def test_run_backtest_realizes_r_target_on_win_minus_one_on_loss_zero_on_timeout():
    n = SAMPLE_SIZE_GATE + 3
    probabilities = np.full(n, 0.9)
    labels = [Barrier.TIMEOUT] * SAMPLE_SIZE_GATE + [Barrier.WIN, Barrier.LOSS, Barrier.TIMEOUT]
    config = ExpectancyGateConfig(r_target=2.0, p_min=0.5, theta=-10.0, cost_r=0.0)

    result = run_backtest(probabilities, labels, config)

    assert result.n_trades == 3
    assert [trade.r_multiple for trade in result.trades] == [2.0, -1.0, 0.0]
    assert [trade.won for trade in result.trades] == [True, False, False]


def test_run_backtest_skips_bars_that_fail_the_gate():
    n = SAMPLE_SIZE_GATE + 2
    probabilities = np.array([0.99] * SAMPLE_SIZE_GATE + [0.99, 0.51])
    labels = [Barrier.WIN] * n
    config = ExpectancyGateConfig(r_target=2.0, p_min=0.55, theta=0.15, cost_r=0.05)

    result = run_backtest(probabilities, labels, config)

    assert result.n_trades == 1
    assert result.trades[0].entry_index == SAMPLE_SIZE_GATE


def test_run_backtest_rejects_mismatched_lengths():
    config = ExpectancyGateConfig(r_target=2.0)
    with pytest.raises(ValueError, match="same length"):
        run_backtest(np.array([0.6, 0.7]), [Barrier.WIN], config)


def test_empty_result_has_zero_expectancy_and_win_rate():
    result = run_backtest(np.array([]), [], ExpectancyGateConfig(r_target=2.0))
    assert result.expectancy == 0.0
    assert result.win_rate == 0.0
    assert result.n_trades == 0


def test_end_to_end_backtest_over_the_synthetic_series_produces_a_sane_result():
    series = generate_labeled_series(n_bars=4000, seed=3)
    y = y_true_from_labels(series.labels)
    split = len(y) * 3 // 4

    model = train_gbdt(series.features[:split], y[:split], TrainConfig(seed=3))
    raw_train = raw_probability(model, series.features[:split])
    raw_test = raw_probability(model, series.features[split:])
    calibrator = IsotonicCalibrator().fit(raw_train, y[:split])
    calibrated_test = calibrator.predict(raw_test)

    config = ExpectancyGateConfig(r_target=R_TARGET)
    result = run_backtest(calibrated_test, series.labels[split:], config)

    assert result.n_candidates == len(series.labels[split:]) - SAMPLE_SIZE_GATE
    # Every taken trade realizes exactly one of the three defined outcomes.
    assert all(trade.r_multiple in (R_TARGET, -1.0, 0.0) for trade in result.trades)
    assert -1.0 <= result.expectancy <= R_TARGET
