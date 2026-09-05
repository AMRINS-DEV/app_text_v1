"""Accelerated-time paper trading and the paper-vs-backtest expectancy
divergence check — §17 Phase 7's literal exit criterion: "60-day paper
expectancy within 1 SE of backtest expectancy"; Prompt 12's "automated
paper-vs-backtest expectancy divergence monitor that halts scaling when
divergence exceeds one standard error."

Sixty days of real paper trading against a live feed is genuinely
impossible to produce in a coding session — no amount of engineering
effort here can make real calendar time pass, and no infrastructure
substitution (the fix for every other "we don't have real X" gap in this
project) closes that particular gap either. What §15 actually checks is a
*statistical property* — does forward performance track backtest
performance — and Phase 2's 72-hour soak test already established this
project's answer to exactly this shape of requirement: simulate the
property via an accelerated, compressed-time run against the same
held-out data and decision pipeline, rather than literally waiting.
"Paper trading" here means running a final fitted model over a genuinely
held-out continuation of the synthetic series that no backtest fold ever
saw — honestly labeled as an accelerated stand-in for elapsed real time,
never claimed as an actual 60-day run.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np
from agents_models.calibration import IsotonicCalibrator
from agents_models.training import TrainConfig, raw_probability, train_gbdt

from .backtest import BacktestResult, ExpectancyGateConfig, run_backtest
from .dataset import LabeledSeries, y_true_from_labels


def run_accelerated_paper_trading(
    series: LabeledSeries,
    *,
    deployment_index: int,
    r_target: float,
    gate_config: ExpectancyGateConfig | None = None,
    train_config: TrainConfig | None = None,
    validation_fraction: float = 0.2,
) -> BacktestResult:
    """Trains a final model on `series[:deployment_index]`, holding out
    the last `validation_fraction` of that range to fit the calibrator
    (never the same data the GBDT trained on — the same discipline
    `run_walk_forward` uses per fold), then runs the real backtest runner
    over `series[deployment_index:]` — a range the training/calibration
    step never saw at all. That trailing range is this module's
    accelerated stand-in for a live paper-trading period."""
    if not 0.0 < validation_fraction < 1.0:
        msg = "validation_fraction must be strictly between 0 and 1"
        raise ValueError(msg)

    y_all = y_true_from_labels(series.labels)
    validate_start = int(deployment_index * (1.0 - validation_fraction))

    x_train = series.features[:validate_start]
    y_train = y_all[:validate_start]
    model = train_gbdt(x_train, y_train, train_config)

    x_validate = series.features[validate_start:deployment_index]
    y_validate = y_all[validate_start:deployment_index]
    raw_validate = raw_probability(model, x_validate)
    calibrator = IsotonicCalibrator().fit(raw_validate, y_validate)

    x_paper = series.features[deployment_index:]
    raw_paper = raw_probability(model, x_paper)
    calibrated_paper = calibrator.predict(raw_paper)

    labels_paper = series.labels[deployment_index:]
    config = gate_config or ExpectancyGateConfig(r_target=r_target)
    return run_backtest(calibrated_paper, labels_paper, config)


@dataclass(frozen=True)
class DivergenceResult:
    paper_expectancy: float
    backtest_expectancy: float
    standard_error: float | None
    divergence: float
    within_one_se: bool
    # `False` when the backtest has too few trades (<2) to estimate a
    # standard error at all — `within_one_se` is forced `False` in that
    # case rather than trivially "passing" against an undefined bound.
    sufficient_evidence: bool

    @property
    def halt_scaling(self) -> bool:
        """Prompt 12: "halts scaling when divergence exceeds one standard
        error." Insufficient evidence halts scaling too — a claim this
        was never actually tested isn't grounds to scale up."""
        return not (self.sufficient_evidence and self.within_one_se)


def check_paper_vs_backtest_divergence(
    paper_result: BacktestResult, backtest_result: BacktestResult
) -> DivergenceResult:
    """§17's own exit criterion, computed for real: is paper expectancy
    within one standard error of backtest expectancy? The standard error
    is of the *backtest* expectancy estimate — the reference distribution
    paper trading is being checked against — per §15's own wording ("live
    expectancy within one standard error of backtest expectancy")."""
    paper_expectancy = paper_result.expectancy
    backtest_expectancy = backtest_result.expectancy
    divergence = abs(paper_expectancy - backtest_expectancy)

    backtest_returns = backtest_result.returns
    if len(backtest_returns) < 2:
        return DivergenceResult(
            paper_expectancy=paper_expectancy,
            backtest_expectancy=backtest_expectancy,
            standard_error=None,
            divergence=divergence,
            within_one_se=False,
            sufficient_evidence=False,
        )

    standard_error = float(np.std(backtest_returns, ddof=1) / np.sqrt(len(backtest_returns)))
    return DivergenceResult(
        paper_expectancy=paper_expectancy,
        backtest_expectancy=backtest_expectancy,
        standard_error=standard_error,
        divergence=divergence,
        within_one_se=divergence <= standard_error,
        sufficient_evidence=True,
    )
