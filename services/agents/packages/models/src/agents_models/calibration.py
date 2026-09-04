"""Probability calibration (§8.3): "Raw GBDT/NN outputs are not
probabilities." Isotonic regression, Brier score, and Expected Calibration
Error — the three things §8.3 says are mandatory before a model's output
may be used for Kelly sizing. Only calibrated probabilities may reach
`domain::Signal.probability` (Rust side); this module is what makes a
probability calibrated in the first place.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import cast

import numpy as np
from numpy.typing import NDArray
from sklearn.isotonic import IsotonicRegression


class IsotonicCalibrator:
    """Wraps `sklearn.isotonic.IsotonicRegression`, clamped to [0,1] output
    (raw isotonic regression can otherwise return exactly 0 or 1, which are
    useless — and dangerous, per §8.5's expectancy gate — as position-sizing
    inputs)."""

    def __init__(self, eps: float = 1e-4) -> None:
        self._model = IsotonicRegression(out_of_bounds="clip", y_min=0.0, y_max=1.0)
        self._eps = eps
        self._fitted = False

    def fit(self, raw_scores: NDArray[np.float64], y_true: NDArray[np.int_]) -> IsotonicCalibrator:
        self._model.fit(raw_scores, y_true)
        self._fitted = True
        return self

    def predict(self, raw_scores: NDArray[np.float64]) -> NDArray[np.float64]:
        if not self._fitted:
            raise RuntimeError("IsotonicCalibrator.fit must be called before predict")
        calibrated = self._model.predict(raw_scores)
        return cast(NDArray[np.float64], np.clip(calibrated, self._eps, 1.0 - self._eps))


def brier_score(y_true: NDArray[np.int_], p_pred: NDArray[np.float64]) -> float:
    """§8.3: "Brier score (lower is better) vs. a base-rate baseline."
    Mean squared error between predicted probability and the {0,1} outcome.
    """
    return float(np.mean((p_pred - y_true) ** 2))


@dataclass(frozen=True)
class CalibrationBin:
    bin_lower: float
    bin_upper: float
    count: int
    mean_predicted: float
    empirical_frequency: float


def reliability_diagram(
    y_true: NDArray[np.int_], p_pred: NDArray[np.float64], n_bins: int = 10
) -> list[CalibrationBin]:
    """§8.3: "Reliability diagram: bucket predictions into deciles; plot
    predicted vs. realized." Empty bins are omitted rather than reported as
    zero — an empty bin isn't miscalibrated, it's just unobserved.
    """
    edges = np.linspace(0.0, 1.0, n_bins + 1)
    bins = []
    for lo, hi in zip(edges[:-1], edges[1:], strict=True):
        # Last bin is closed on both ends so p_pred == 1.0 lands somewhere.
        in_bin = (p_pred >= lo) & (p_pred < hi if hi < 1.0 else p_pred <= hi)
        count = int(in_bin.sum())
        if count == 0:
            continue
        bins.append(
            CalibrationBin(
                bin_lower=float(lo),
                bin_upper=float(hi),
                count=count,
                mean_predicted=float(p_pred[in_bin].mean()),
                empirical_frequency=float(y_true[in_bin].mean()),
            )
        )
    return bins


def expected_calibration_error(
    y_true: NDArray[np.int_], p_pred: NDArray[np.float64], n_bins: int = 10
) -> float:
    """§8.3: "Expected Calibration Error (ECE) target < 0.03." Weighted mean
    absolute gap between each bin's average prediction and its empirical
    win rate, weighted by bin population.
    """
    bins = reliability_diagram(y_true, p_pred, n_bins)
    total = len(p_pred)
    if total == 0 or not bins:
        return 0.0
    return sum(b.count * abs(b.mean_predicted - b.empirical_frequency) for b in bins) / total


def max_calibration_gap(
    y_true: NDArray[np.int_], p_pred: NDArray[np.float64], n_bins: int = 10
) -> float:
    """§8.3: "Deviation >5pp in any populated bucket = not deployable." The
    per-bucket counterpart to ECE's population-weighted average — a model
    can have a fine ECE overall while still having one badly miscalibrated
    bucket that ECE's averaging hides.
    """
    bins = reliability_diagram(y_true, p_pred, n_bins)
    if not bins:
        return 0.0
    return max(abs(b.mean_predicted - b.empirical_frequency) for b in bins)
