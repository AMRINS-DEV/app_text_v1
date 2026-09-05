import numpy as np
import pytest
from agents_validation.statistics import deflated_sharpe_ratio, monte_carlo_drawdown_distribution
from scipy import stats


def test_deflated_sharpe_ratio_is_a_probability():
    rng = np.random.default_rng(1)
    returns = rng.normal(loc=0.05, scale=1.0, size=200)
    dsr = deflated_sharpe_ratio(returns, n_trials=1)
    assert 0.0 <= dsr <= 1.0


def test_a_clearly_skilled_strategy_gets_a_high_dsr():
    rng = np.random.default_rng(2)
    returns = rng.normal(loc=1.0, scale=0.3, size=500)  # strong, consistent positive edge
    dsr = deflated_sharpe_ratio(returns, n_trials=1)
    assert dsr > 0.99


def test_a_clearly_unskilled_strategy_gets_a_low_dsr():
    rng = np.random.default_rng(3)
    returns = rng.normal(loc=-1.0, scale=0.3, size=500)  # consistent negative edge
    dsr = deflated_sharpe_ratio(returns, n_trials=1)
    assert dsr < 0.01


def test_dsr_is_uniformly_distributed_across_trials_when_the_true_sharpe_is_zero():
    # DSR is a normal CDF applied to a (re-standardized) sample statistic,
    # so under the null (true SR=0) it behaves like a calibrated p-value:
    # uniformly distributed on [0, 1] -- *not* concentrated near 0.5 for
    # any single noisy draw (a single realization can land anywhere in
    # [0, 1] and that is correct, not a bug). The invariant that actually
    # holds is on the mean *across many independent trials*, which a
    # Uniform[0,1] puts at 0.5.
    rng = np.random.default_rng(4)
    dsr_values = [
        deflated_sharpe_ratio(rng.normal(loc=0.0, scale=1.0, size=200), n_trials=1)
        for _ in range(300)
    ]
    assert np.mean(dsr_values) == pytest.approx(0.5, abs=0.08)


def test_more_trials_never_increases_the_deflated_sharpe_ratio():
    # More trials raises the bar (expected max Sharpe under the null grows
    # with n_trials), so DSR on the *same* returns can only fall or stay flat.
    rng = np.random.default_rng(5)
    returns = rng.normal(loc=0.3, scale=1.0, size=300)
    dsr_1 = deflated_sharpe_ratio(returns, n_trials=1)
    dsr_10 = deflated_sharpe_ratio(returns, n_trials=10)
    dsr_1000 = deflated_sharpe_ratio(returns, n_trials=1000)
    assert dsr_1 >= dsr_10 >= dsr_1000


def test_matches_the_closed_form_probabilistic_sharpe_ratio_for_near_normal_returns():
    # For skew=0, kurtosis=3 (normal), Var[SR_hat] reduces exactly to
    # 1/(n-1), and with n_trials=1 (benchmark=0) the formula collapses to
    # the textbook PSR: Phi(SR * sqrt(n-1)). Large-n normal samples have
    # sample skew/kurtosis close enough to (0, 3) for this to hold to a
    # loose tolerance -- an independent cross-check of the general formula
    # against its own well-known simplified special case.
    rng = np.random.default_rng(6)
    returns = rng.normal(loc=0.1, scale=1.0, size=5000)
    sharpe = np.mean(returns) / np.std(returns, ddof=1)
    expected = stats.norm.cdf(sharpe * np.sqrt(len(returns) - 1))

    dsr = deflated_sharpe_ratio(returns, n_trials=1)

    assert dsr == pytest.approx(expected, abs=0.02)


def test_degenerate_inputs_do_not_crash():
    assert deflated_sharpe_ratio(np.array([]), n_trials=1) == 0.0
    assert deflated_sharpe_ratio(np.array([1.0]), n_trials=1) == 0.0
    assert deflated_sharpe_ratio(np.zeros(50), n_trials=1) == 0.0


def test_monte_carlo_drawdown_is_deterministic_for_the_same_seed():
    returns = np.array([1.0, -2.0, 3.0, -1.0, 0.5, -3.0, 2.0])
    a = monte_carlo_drawdown_distribution(returns, n_simulations=200, seed=42)
    b = monte_carlo_drawdown_distribution(returns, n_simulations=200, seed=42)
    assert np.array_equal(a.simulations, b.simulations)


def test_monte_carlo_drawdown_differs_across_seeds():
    returns = np.array([1.0, -2.0, 3.0, -1.0, 0.5, -3.0, 2.0])
    a = monte_carlo_drawdown_distribution(returns, n_simulations=200, seed=1)
    b = monte_carlo_drawdown_distribution(returns, n_simulations=200, seed=2)
    assert not np.array_equal(a.simulations, b.simulations)


def test_all_positive_returns_never_draw_down_regardless_of_order():
    returns = np.array([1.0, 2.0, 3.0, 0.5, 4.0])
    dist = monte_carlo_drawdown_distribution(returns, n_simulations=300, seed=7)
    assert dist.worst == 0.0
    assert dist.median == 0.0


def test_all_negative_returns_always_draw_down_by_the_same_fixed_amount():
    returns = np.array([-1.0, -2.0, -3.0])
    dist = monte_carlo_drawdown_distribution(returns, n_simulations=300, seed=7)
    expected = -returns.sum()
    assert np.allclose(dist.simulations, expected)


def test_mixed_returns_produce_a_genuinely_varying_distribution_under_reshuffling():
    rng = np.random.default_rng(8)
    returns = rng.normal(loc=0.05, scale=1.0, size=50)
    dist = monte_carlo_drawdown_distribution(returns, n_simulations=500, seed=9)
    assert np.std(dist.simulations) > 0.0
    assert dist.p95 >= dist.median


def test_n_simulations_controls_the_sample_count():
    dist = monte_carlo_drawdown_distribution(np.array([1.0, -1.0]), n_simulations=37, seed=1)
    assert len(dist.simulations) == 37


def test_empty_returns_gives_a_zero_distribution():
    dist = monte_carlo_drawdown_distribution(np.array([]), n_simulations=10, seed=1)
    assert dist.median == 0.0
    assert dist.worst == 0.0
