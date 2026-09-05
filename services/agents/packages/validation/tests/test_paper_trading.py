import numpy as np
import pytest
from agents_validation import generate_labeled_series
from agents_validation.backtest import BacktestResult, BacktestTrade
from agents_validation.paper_trading import (
    check_paper_vs_backtest_divergence,
    run_accelerated_paper_trading,
)
from agents_validation.walk_forward import run_walk_forward

R_TARGET = 2.2 / 1.5  # matches the dataset's own default tp_mult/sl_mult ratio


def _backtest_result(r_multiples: list[float]) -> BacktestResult:
    trades = [
        BacktestTrade(entry_index=i, probability=0.6, r_multiple=r, won=r > 0)
        for i, r in enumerate(r_multiples)
    ]
    return BacktestResult(trades=trades, n_candidates=len(r_multiples))


def test_run_accelerated_paper_trading_produces_a_well_formed_result():
    series = generate_labeled_series(n_bars=4000, seed=3)
    result = run_accelerated_paper_trading(
        series, deployment_index=3000, r_target=R_TARGET
    )

    # The paper leg only ever evaluates series[deployment_index:], so it can
    # never see more candidate bars than that range holds.
    assert result.n_candidates <= len(series.labels) - 3000
    assert all(trade.r_multiple in (R_TARGET, -1.0, 0.0) for trade in result.trades)
    assert -1.0 <= result.expectancy <= R_TARGET


@pytest.mark.parametrize("validation_fraction", [0.0, 1.0, -0.1, 1.5])
def test_validation_fraction_out_of_range_raises(validation_fraction):
    series = generate_labeled_series(n_bars=4000, seed=3)
    with pytest.raises(ValueError, match="validation_fraction"):
        run_accelerated_paper_trading(
            series,
            deployment_index=3000,
            r_target=R_TARGET,
            validation_fraction=validation_fraction,
        )


def test_divergence_reports_insufficient_evidence_with_zero_backtest_trades():
    paper = _backtest_result([1.0, 1.0, -1.0])
    backtest = _backtest_result([])

    divergence = check_paper_vs_backtest_divergence(paper, backtest)

    assert divergence.sufficient_evidence is False
    assert divergence.within_one_se is False
    assert divergence.standard_error is None
    assert divergence.halt_scaling is True


def test_divergence_reports_insufficient_evidence_with_one_backtest_trade():
    paper = _backtest_result([1.0])
    backtest = _backtest_result([2.2])

    divergence = check_paper_vs_backtest_divergence(paper, backtest)

    assert divergence.sufficient_evidence is False
    assert divergence.within_one_se is False
    assert divergence.halt_scaling is True


def test_divergence_within_one_se_does_not_halt_scaling():
    # backtest returns [1, 1, -1, -1]: mean=0, sample std=sqrt(4/3), SE=std/2.
    backtest = _backtest_result([1.0, 1.0, -1.0, -1.0])
    paper = _backtest_result([0.3, 0.3, 0.3])  # paper expectancy = 0.3

    divergence = check_paper_vs_backtest_divergence(paper, backtest)

    expected_se = float(np.std([1.0, 1.0, -1.0, -1.0], ddof=1) / np.sqrt(4))
    assert divergence.backtest_expectancy == pytest.approx(0.0)
    assert divergence.paper_expectancy == pytest.approx(0.3)
    assert divergence.standard_error == pytest.approx(expected_se)
    assert divergence.divergence == pytest.approx(0.3)
    assert divergence.sufficient_evidence is True
    assert divergence.within_one_se is True
    assert divergence.halt_scaling is False


def test_divergence_beyond_one_se_halts_scaling():
    backtest = _backtest_result([1.0, 1.0, -1.0, -1.0])  # SE ~= 0.577
    paper = _backtest_result([2.0])  # divergence = 2.0, well past the SE

    divergence = check_paper_vs_backtest_divergence(paper, backtest)

    assert divergence.sufficient_evidence is True
    assert divergence.within_one_se is False
    assert divergence.halt_scaling is True


def test_end_to_end_walk_forward_backtest_vs_accelerated_paper_trading():
    # Wires the two reference implementations together exactly as a real
    # deployment would: the walk-forward result over the pre-deployment
    # history stands in for "backtest expectancy," and the accelerated
    # paper leg over the trailing, never-backtested continuation stands in
    # for "60-day paper expectancy" -- the literal §17 exit criterion. This
    # asserts the check runs and produces well-formed output; it does not
    # rig the synthetic data to force a pass.
    series = generate_labeled_series(n_bars=4000, seed=3)
    deployment_index = 3000

    backtest = run_walk_forward(
        series,
        train_periods=1200,
        validate_periods=300,
        test_periods=300,
        r_target=R_TARGET,
    )
    # WalkForwardResult has no n_candidates/trades in BacktestResult shape;
    # adapt it into one so the same divergence check applies to either leg.
    backtest_result = _backtest_result([t.r_multiple for t in backtest.all_trades])

    paper_result = run_accelerated_paper_trading(
        series, deployment_index=deployment_index, r_target=R_TARGET
    )

    divergence = check_paper_vs_backtest_divergence(paper_result, backtest_result)

    assert isinstance(divergence.halt_scaling, bool)
    assert divergence.divergence >= 0.0
    if divergence.sufficient_evidence:
        assert divergence.standard_error is not None
        assert divergence.standard_error >= 0.0
