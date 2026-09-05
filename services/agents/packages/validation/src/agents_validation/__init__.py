"""The §15/§17 Phase 7 validation pipeline (§17 exit row: "Walk-forward,
paper trading, calibration monitoring | 60-day paper expectancy within 1
SE of backtest"). See each submodule's own doc comment for what it wires
together and what's synthetic vs. real underneath.
"""

from .backtest import (
    DEFAULT_COST_R,
    SAMPLE_SIZE_GATE,
    BacktestResult,
    BacktestTrade,
    ExpectancyGateConfig,
    expected_r,
    passes_expectancy_gate,
    run_backtest,
)
from .calibration_monitor import CalibrationSnapshot, RollingCalibrationMonitor
from .dataset import LabeledSeries, generate_labeled_series, y_true_from_labels
from .paper_trading import (
    DivergenceResult,
    check_paper_vs_backtest_divergence,
    run_accelerated_paper_trading,
)
from .statistics import (
    DrawdownDistribution,
    deflated_sharpe_ratio,
    monte_carlo_drawdown_distribution,
)
from .walk_forward import WalkForwardFoldResult, WalkForwardResult, run_walk_forward

__all__ = [
    "LabeledSeries",
    "generate_labeled_series",
    "y_true_from_labels",
    "DEFAULT_COST_R",
    "SAMPLE_SIZE_GATE",
    "BacktestResult",
    "BacktestTrade",
    "ExpectancyGateConfig",
    "expected_r",
    "passes_expectancy_gate",
    "run_backtest",
    "WalkForwardFoldResult",
    "WalkForwardResult",
    "run_walk_forward",
    "DrawdownDistribution",
    "deflated_sharpe_ratio",
    "monte_carlo_drawdown_distribution",
    "CalibrationSnapshot",
    "RollingCalibrationMonitor",
    "DivergenceResult",
    "check_paper_vs_backtest_divergence",
    "run_accelerated_paper_trading",
]
