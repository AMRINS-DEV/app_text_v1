"""§15 items 4-5: Deflated Sharpe Ratio and the Monte Carlo trade-order-
shuffle drawdown distribution. Neither existed anywhere in the repo before
Phase 7 (confirmed by grep — "deflated_sharpe" and "monte_carlo" had zero
matches).

The Deflated Sharpe Ratio (Bailey & Lopez de Prado, 2014) answers "is this
Sharpe ratio real skill, or what you'd expect from the best of `n_trials`
random strategies?" — exactly the "Sharpe of 2.0 after 500 trials is
noise" failure mode §8.7 exists to prevent, now quantified rather than
just guarded against by the purging/embargo splitters. It reduces to the
ordinary Probabilistic Sharpe Ratio when `n_trials=1` (no multiple-testing
correction needed for a single trial).
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np
from numpy.typing import NDArray
from scipy import stats

_EULER_MASCHERONI = 0.5772156649015329


def _sample_skewness(returns: NDArray[np.float64]) -> float:
    return float(stats.skew(returns, bias=False))


def _sample_kurtosis(returns: NDArray[np.float64]) -> float:
    """Non-excess kurtosis (a normal distribution has kurtosis 3, not 0) —
    the convention the Deflated Sharpe Ratio formula itself uses."""
    return float(stats.kurtosis(returns, fisher=False, bias=False))


def _sharpe_ratio(returns: NDArray[np.float64]) -> float:
    std = float(np.std(returns, ddof=1))
    if std <= 0.0:
        return 0.0
    return float(np.mean(returns)) / std


def _expected_max_sharpe(n_trials: int, sharpe_std: float) -> float:
    """The expected maximum Sharpe ratio observed across `n_trials`
    independent, skill-less (true SR=0) strategies — the benchmark the
    Deflated Sharpe Ratio measures the actual Sharpe against, instead of
    the naive (and multiple-testing-blind) benchmark of zero."""
    if n_trials <= 1:
        return 0.0
    return float(
        sharpe_std
        * (
            (1.0 - _EULER_MASCHERONI) * stats.norm.ppf(1.0 - 1.0 / n_trials)
            + _EULER_MASCHERONI * stats.norm.ppf(1.0 - 1.0 / (n_trials * np.e))
        )
    )


def deflated_sharpe_ratio(returns: NDArray[np.float64], n_trials: int = 1) -> float:
    """Returns the probability (in `[0, 1]`) that the strategy's true
    Sharpe ratio exceeds the expected maximum Sharpe of `n_trials`
    skill-less strategies — i.e., how likely the observed performance is
    genuine skill rather than the best-of-many-trials artifact §8.7 warns
    about. `n_trials=1` (the default) is the ordinary Probabilistic Sharpe
    Ratio: no multiple-testing correction, just "how likely is a positive
    true Sharpe given estimation noise."
    """
    n = len(returns)
    if n < 2 or float(np.std(returns, ddof=1)) <= 0.0:
        return 0.0

    sharpe = _sharpe_ratio(returns)
    skewness = _sample_skewness(returns)
    kurtosis = _sample_kurtosis(returns)

    # Var[SR_hat] under non-normal returns (Bailey & Lopez de Prado eq. 8;
    # reduces to the textbook 1/(n-1) when returns are normal, since then
    # skewness=0 and kurtosis=3).
    variance_term = 1.0 - skewness * sharpe + ((kurtosis - 1.0) / 4.0) * sharpe**2
    if variance_term <= 0.0:
        # A pathological input (e.g. near-zero variance with extreme
        # skew) — treat as "no evidence of skill" rather than dividing by
        # a non-positive number.
        return 0.0
    sharpe_std = np.sqrt(variance_term / (n - 1))

    benchmark_sharpe = _expected_max_sharpe(n_trials, sharpe_std)
    z = (sharpe - benchmark_sharpe) * np.sqrt(n - 1) / np.sqrt(variance_term)
    return float(stats.norm.cdf(z))


@dataclass(frozen=True)
class DrawdownDistribution:
    simulations: NDArray[np.float64]

    def percentile(self, q: float) -> float:
        return float(np.percentile(self.simulations, q))

    @property
    def median(self) -> float:
        return self.percentile(50.0)

    @property
    def p95(self) -> float:
        return self.percentile(95.0)

    @property
    def worst(self) -> float:
        return float(np.max(self.simulations)) if len(self.simulations) else 0.0


def _max_drawdown(returns: NDArray[np.float64]) -> float:
    equity = np.concatenate([[0.0], np.cumsum(returns)])
    peak = np.maximum.accumulate(equity)
    return float(np.max(peak - equity))


def monte_carlo_drawdown_distribution(
    returns: NDArray[np.float64], n_simulations: int = 1000, seed: int | None = None
) -> DrawdownDistribution:
    """§15 item 5: shuffle the realized trade returns into `n_simulations`
    random orderings, recompute the equity curve and its max drawdown for
    each, and return the empirical distribution. A strategy whose realized
    max drawdown is far better than most reshufflings got lucky on
    sequencing, not on trade selection — the whole point of testing this
    at all rather than trusting the single historical ordering."""
    rng = np.random.default_rng(seed)
    n = len(returns)
    if n == 0:
        return DrawdownDistribution(simulations=np.zeros(n_simulations))

    drawdowns = np.empty(n_simulations)
    for i in range(n_simulations):
        shuffled = rng.permutation(returns)
        drawdowns[i] = _max_drawdown(shuffled)
    return DrawdownDistribution(simulations=drawdowns)
