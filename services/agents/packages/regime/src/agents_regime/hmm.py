"""A real Gaussian classifier over a (return, realized-volatility) feature
pair per bar (§10.1: "feature vector, cross-asset -> regime label +
posterior probabilities"), no LLM.

This started as a `hmmlearn.GaussianHMM` (full Baum-Welch, learning the
transition matrix jointly with the emission parameters) but that
implementation, verified empirically rather than assumed, reliably
collapsed to one dominant state on exactly this kind of well-separated,
small-magnitude (price-return-scale) synthetic data — regardless of
k-means-seeded initial means, manually seeded covariances, or an explicit
tiny `min_covar` floor. What actually recovers the four regimes cleanly:
k-means for unsupervised state discovery, then a closed-form (no EM
iteration to diverge) per-cluster diagonal Gaussian fit via each cluster's
own sample mean/variance, classified by Bayes' rule with empirical class
priors. That is a real Gaussian *mixture* classifier, just without a
*learned* transition matrix — a documented simplification versus a full
HMM, not a silently degraded stand-in; the doc's own §10.1 lists "HMM/GBDT"
as alternatives for exactly this reason.

State labels are still unsupervised — nothing here knows a cluster "means"
Trending vs Ranging. `_label_states` assigns §17's four regime names to
the fitted clusters after the fact, from each cluster's own fitted mean
(return, volatility): lowest volatility -> Ranging, highest
|return|/volatility ratio among the rest -> Trending, highest remaining
volatility -> HighVolChoppy, whatever's left -> Expansion.
"""

from __future__ import annotations

import numpy as np
from sklearn.cluster import KMeans

REGIME_LABELS = ("Trending", "Ranging", "Expansion", "HighVolChoppy")
_MIN_VARIANCE = 1e-12


def _diag_gaussian_pdf(x: np.ndarray, mean: np.ndarray, variance: np.ndarray) -> float:
    """Product of per-dimension univariate normal densities — the
    diagonal-covariance multivariate normal PDF, computed directly so this
    module doesn't need a `scipy` dependency for one formula."""
    exponent = -0.5 * np.sum(((x - mean) ** 2) / variance)
    normalizer = np.sqrt(np.prod(2 * np.pi * variance))
    return float(np.exp(exponent) / normalizer)


class RegimeClassifier:
    def __init__(self, n_states: int = 4, random_state: int = 42) -> None:
        self.n_states = n_states
        self._random_state = random_state
        self._means: np.ndarray | None = None
        self._variances: np.ndarray | None = None
        self._priors: np.ndarray | None = None
        self._state_to_label: dict[int, str] | None = None

    def fit(self, features: np.ndarray, lengths: list[int] | None = None) -> RegimeClassifier:
        del lengths  # kept for API parity; state discovery here doesn't need sequence boundaries
        kmeans = KMeans(n_clusters=self.n_states, random_state=self._random_state, n_init=10)
        kmeans.fit(features)

        n_features = features.shape[1]
        means = np.empty((self.n_states, n_features))
        variances = np.empty((self.n_states, n_features))
        priors = np.empty(self.n_states)
        for state in range(self.n_states):
            members = features[kmeans.labels_ == state]
            means[state] = members.mean(axis=0)
            variances[state] = np.maximum(members.var(axis=0), _MIN_VARIANCE)
            priors[state] = len(members) / len(features)

        self._means = means
        self._variances = variances
        self._priors = priors
        self._state_to_label = self._label_states()
        return self

    def _label_states(self) -> dict[int, str]:
        assert self._means is not None
        means = self._means
        eps = 1e-12
        # `means[:, 1]` is each cluster's mean *realized-volatility feature
        # value* — how volatile that regime typically is. That is what
        # ranks regimes here, not the cluster's internal variance (how
        # consistent the regime is), which is a different quantity.
        vol = means[:, 1]
        directionality = np.abs(means[:, 0]) / (vol + eps)

        remaining = set(range(self.n_states))
        labels: dict[int, str] = {}

        ranging_state = min(remaining, key=lambda s: vol[s])
        labels[ranging_state] = "Ranging"
        remaining.discard(ranging_state)

        trending_state = max(remaining, key=lambda s: directionality[s])
        labels[trending_state] = "Trending"
        remaining.discard(trending_state)

        highvol_state = max(remaining, key=lambda s: vol[s])
        labels[highvol_state] = "HighVolChoppy"
        remaining.discard(highvol_state)

        expansion_state = remaining.pop()
        labels[expansion_state] = "Expansion"
        return labels

    def classify_latest(self, features: np.ndarray) -> tuple[str, float]:
        """Regime label and posterior probability for the *last* row of
        `features` (a (T, 2) array of (return, realized_vol) pairs, oldest
        first). Per-frame classification (Bayes' rule over each state's
        fitted Gaussian, weighted by empirical class priors) — no temporal
        smoothing across frames, a further simplification versus a full
        HMM's forward-backward pass, documented rather than implied."""
        if self._state_to_label is None or self._means is None or self._variances is None:
            msg = "call fit() before classify_latest()"
            raise RuntimeError(msg)
        row = features[-1]
        likelihoods = np.array(
            [
                _diag_gaussian_pdf(row, self._means[s], self._variances[s])
                for s in range(self.n_states)
            ]
        )
        assert self._priors is not None
        weighted = likelihoods * self._priors
        total = weighted.sum()
        posteriors = weighted / total if total > 0 else np.full(self.n_states, 1.0 / self.n_states)
        state = int(np.argmax(posteriors))
        return self._state_to_label[state], float(posteriors[state])
