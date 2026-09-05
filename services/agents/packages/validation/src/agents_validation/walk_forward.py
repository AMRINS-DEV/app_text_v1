"""Walk-forward orchestration (§8.7's "rolling train -> validate -> test,
step forward, never reuse test data"; §17 Phase 7's own "Walk-forward"
exit-row scope): for each `WalkForwardWindow` from
`agents_models.cross_validation.walk_forward_windows`, train a GBDT on the
train range, fit the isotonic calibrator on the (held out from training)
validate range, then run the real backtest runner over the test range —
the only range genuinely out-of-sample for both the model and its
calibration. Nothing in Phase 3 wired training, calibration, and CV
splitting together into an actual walk; this closes that gap.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np
from agents_models.calibration import IsotonicCalibrator
from agents_models.cross_validation import WalkForwardWindow, walk_forward_windows
from agents_models.training import TrainConfig, raw_probability, train_gbdt
from numpy.typing import NDArray

from .backtest import BacktestResult, BacktestTrade, ExpectancyGateConfig, run_backtest
from .dataset import LabeledSeries, y_true_from_labels


@dataclass(frozen=True)
class WalkForwardFoldResult:
    window: WalkForwardWindow
    backtest: BacktestResult
    calibrated_probabilities: NDArray[np.float64]
    y_true: NDArray[np.int_]


@dataclass(frozen=True)
class WalkForwardResult:
    folds: list[WalkForwardFoldResult]

    @property
    def all_trades(self) -> list[BacktestTrade]:
        return [trade for fold in self.folds for trade in fold.backtest.trades]

    @property
    def expectancy(self) -> float:
        """Mean realized R-multiple pooled across every fold's out-of-
        sample trades — the single number §17's "60-day paper expectancy
        within 1 SE of backtest" exit criterion compares against."""
        trades = self.all_trades
        if not trades:
            return 0.0
        return float(np.mean([trade.r_multiple for trade in trades]))

    @property
    def returns(self) -> NDArray[np.float64]:
        return np.array([trade.r_multiple for trade in self.all_trades], dtype=np.float64)


def run_walk_forward(
    series: LabeledSeries,
    *,
    train_periods: int,
    validate_periods: int,
    test_periods: int,
    r_target: float,
    gate_config: ExpectancyGateConfig | None = None,
    train_config: TrainConfig | None = None,
) -> WalkForwardResult:
    windows = walk_forward_windows(
        len(series.labels), train_periods, validate_periods, test_periods
    )
    if not windows:
        msg = "series too short for the requested train/validate/test period lengths"
        raise ValueError(msg)

    y_all = y_true_from_labels(series.labels)
    config = gate_config or ExpectancyGateConfig(r_target=r_target)

    folds: list[WalkForwardFoldResult] = []
    for window in windows:
        x_train = series.features[window.train_start : window.train_end]
        y_train = y_all[window.train_start : window.train_end]
        model = train_gbdt(x_train, y_train, train_config)

        x_validate = series.features[window.validate_start : window.validate_end]
        y_validate = y_all[window.validate_start : window.validate_end]
        raw_validate = raw_probability(model, x_validate)
        calibrator = IsotonicCalibrator().fit(raw_validate, y_validate)

        x_test = series.features[window.test_start : window.test_end]
        y_test = y_all[window.test_start : window.test_end]
        raw_test = raw_probability(model, x_test)
        calibrated_test = calibrator.predict(raw_test)

        labels_test = series.labels[window.test_start : window.test_end]
        backtest = run_backtest(calibrated_test, labels_test, config)

        folds.append(
            WalkForwardFoldResult(
                window=window,
                backtest=backtest,
                calibrated_probabilities=calibrated_test,
                y_true=y_test,
            )
        )

    return WalkForwardResult(folds=folds)
