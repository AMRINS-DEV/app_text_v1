from __future__ import annotations

import numpy as np
import pytest
from agents_regime.hmm import RegimeClassifier

# (mean_return, std_return, mean_vol, std_vol) per regime — deliberately
# well-separated so a correctly-fit HMM should recover them cleanly; this
# is a synthetic dataset, not real market data (consistent with Phase 3's
# ML training data being synthetic for the same reason: no ingested tick
# archive exists in this environment).
_SPECS: dict[str, tuple[float, float, float, float]] = {
    "Trending": (0.0020, 0.0004, 0.0006, 0.0001),
    "Ranging": (0.0000, 0.0002, 0.0003, 0.00005),
    "Expansion": (0.0000, 0.0010, 0.0012, 0.0002),
    "HighVolChoppy": (0.0000, 0.0020, 0.0025, 0.0003),
}


def _segment(rng: np.random.Generator, regime: str, n: int) -> np.ndarray:
    mean_r, std_r, mean_v, std_v = _SPECS[regime]
    returns = rng.normal(mean_r, std_r, n)
    vols = np.abs(rng.normal(mean_v, std_v, n))
    return np.column_stack([returns, vols])


@pytest.fixture
def fitted_classifier() -> tuple[RegimeClassifier, dict[str, np.ndarray]]:
    rng = np.random.default_rng(7)
    n = 300
    segments = {regime: _segment(rng, regime, n) for regime in _SPECS}
    order = ["Trending", "Ranging", "Expansion", "HighVolChoppy"]
    features = np.concatenate([segments[r] for r in order])
    lengths = [n] * len(order)

    classifier = RegimeClassifier(n_states=4, random_state=7).fit(features, lengths)
    return classifier, segments


def test_classify_latest_requires_fit_first():
    classifier = RegimeClassifier(n_states=4)
    with pytest.raises(RuntimeError):
        classifier.classify_latest(np.zeros((5, 2)))


@pytest.mark.parametrize("regime", list(_SPECS))
def test_recovers_the_correct_regime_for_a_held_out_sample(fitted_classifier, regime):
    classifier, segments = fitted_classifier
    held_out = segments[regime][:50]  # a prefix, standing in for "observed so far"
    label, posterior = classifier.classify_latest(held_out)
    assert label == regime, f"expected {regime}, got {label} (posterior {posterior})"
    assert 0.0 <= posterior <= 1.0


def test_posterior_is_reasonably_confident_on_well_separated_synthetic_data(fitted_classifier):
    classifier, segments = fitted_classifier
    _, posterior = classifier.classify_latest(segments["Trending"][:50])
    assert posterior > 0.6
