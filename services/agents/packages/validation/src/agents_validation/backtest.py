"""The backtest runner (§8.5's expectancy gate, §15's "backtest with
realistic costs"): given a fitted, calibrated model's output and a labeled
time series, walks forward bar-by-bar, applies the exact §8.5 formula to
decide whether to take each trade, and resolves taken trades via their
already-computed triple-barrier outcome — the orchestration nothing in
Phase 3 built (see this package's own `__init__.py` doc comment): the
GBDT model, the isotonic calibrator, and a labeled time series previously
had no code connecting them into an actual trade sequence.

Deliberately not NautilusTrader, the specific engine §15 names: this
project already built its own simulated-execution stack in Phase 2
(`SimBroker`, `OrderRouter`, `crates/risk`'s Kelly sizing and guard suite)
purpose-built for exactly this job. Bolting on a second, unrelated
backtesting framework — whose own strategy/execution model would bypass
everything Phases 1-6 built, and reimplement the same trade logic a
second time disconnected from the Rust engine — is the same call Phase 6
made about LangGraph: a large dependency for behavior this codebase can
already produce with what it has. See the README's Phase 7 section for
the full reasoning. "Realistic costs" (§15 item 3) are represented as a
fixed cost-in-R constant — the same "cost ceiling" §8.5's own formula
already accounts for, not a full market-impact model.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np
from agents_models.labeling import Barrier
from numpy.typing import NDArray

# Mirrors crates/strategy::fusion::SAMPLE_SIZE_GATE and
# agents_graph.priors.SAMPLE_SIZE_GATE — the same "don't trust a source
# with too little history" convention applied here to the backtest's own
# warm-up period.
SAMPLE_SIZE_GATE = 30

# Spread + slippage, in units of R. A documented constant standing in for
# a real market-impact model, same "real logic, mock infrastructure"
# split as SimBroker's own fixed slippage_points.
DEFAULT_COST_R = 0.05


@dataclass(frozen=True)
class ExpectancyGateConfig:
    r_target: float
    p_min: float = 0.55
    theta: float = 0.15
    cost_r: float = DEFAULT_COST_R


def expected_r(probability: float, config: ExpectancyGateConfig) -> float:
    """§8.5: `E[R] = p*R_target - (1-p)*1.0 - c`."""
    return probability * config.r_target - (1.0 - probability) * 1.0 - config.cost_r


def passes_expectancy_gate(probability: float, config: ExpectancyGateConfig) -> bool:
    """§8.5's three conditions: cost ceiling, minimum probability, and the
    expectancy threshold itself — all three must hold, in that order (the
    cost ceiling is a property of the setup, not the bar, so checking it
    first avoids computing `expected_r` for a setup that can never pass)."""
    if config.cost_r > 0.10 * config.r_target:
        return False
    if probability < config.p_min:
        return False
    return expected_r(probability, config) >= config.theta


@dataclass(frozen=True)
class BacktestTrade:
    entry_index: int
    probability: float
    r_multiple: float
    won: bool


@dataclass(frozen=True)
class BacktestResult:
    trades: list[BacktestTrade]
    n_candidates: int  # bars past the warm-up where a decision was evaluated at all

    @property
    def n_trades(self) -> int:
        return len(self.trades)

    @property
    def expectancy(self) -> float:
        """Mean realized R-multiple across taken trades."""
        if not self.trades:
            return 0.0
        return float(np.mean([trade.r_multiple for trade in self.trades]))

    @property
    def win_rate(self) -> float:
        if not self.trades:
            return 0.0
        return sum(1 for trade in self.trades if trade.won) / len(self.trades)

    @property
    def returns(self) -> NDArray[np.float64]:
        return np.array([trade.r_multiple for trade in self.trades], dtype=np.float64)


def run_backtest(
    calibrated_probabilities: NDArray[np.float64],
    labels: list[Barrier],
    config: ExpectancyGateConfig,
) -> BacktestResult:
    """Walks the series bar by bar. `calibrated_probabilities[i]` is the
    already-calibrated P(win) for the primary long bet at bar `i`;
    `labels[i]` is that bar's already-resolved triple-barrier outcome. A
    trade taken at a WIN bar realizes `+config.r_target`; at a LOSS bar,
    `-1.0`; at a TIMEOUT bar, `0.0` (flat — resolved by time running out,
    not by price). The first `SAMPLE_SIZE_GATE` bars never trade
    regardless of probability, the same warm-up every other source in
    this project's fusion/priors machinery gets."""
    if len(calibrated_probabilities) != len(labels):
        msg = "calibrated_probabilities and labels must be the same length"
        raise ValueError(msg)

    trades: list[BacktestTrade] = []
    n_candidates = 0

    for i, (probability, label) in enumerate(zip(calibrated_probabilities, labels, strict=True)):
        if i < SAMPLE_SIZE_GATE:
            continue
        n_candidates += 1
        if not passes_expectancy_gate(float(probability), config):
            continue
        if label == Barrier.WIN:
            r_multiple, won = config.r_target, True
        elif label == Barrier.LOSS:
            r_multiple, won = -1.0, False
        else:
            r_multiple, won = 0.0, False
        trades.append(
            BacktestTrade(
                entry_index=i, probability=float(probability), r_multiple=r_multiple, won=won
            )
        )

    return BacktestResult(trades=trades, n_candidates=n_candidates)
