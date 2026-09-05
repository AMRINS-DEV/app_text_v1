"""A rolling calibration drift monitor (§17 Phase 7's own "calibration
monitoring" exit-row scope): wraps `agents_models.calibration`'s Brier
score / ECE — real, tested metrics with no orchestration over a live
stream anywhere in Phase 3 — in a sliding window, flagging when
calibration degrades past §8.3's own target as new
`(predicted_probability, realized_outcome)` pairs arrive one at a time,
the same shape a live paper-trading or production feed would produce
them in.
"""

from __future__ import annotations

from collections import deque
from dataclasses import dataclass

import numpy as np
from agents_models.calibration import brier_score, expected_calibration_error
from numpy.typing import NDArray

# §8.3's own calibration target is ECE < 0.03; drift is flagged only once
# a window's ECE clears a materially worse bar than that target, not the
# target itself (a single window bouncing right around 0.03 shouldn't
# page anyone). The window size matters for this threshold to mean
# anything: ECE over a small window is dominated by binomial sampling
# noise even for a genuinely well-calibrated model (empirically, a
# window of 200 crosses 0.05 most of the time even when perfectly
# calibrated — verified while tuning these defaults). At 1000, a
# well-calibrated stream stays under 0.05 about 99% of the time.
DEFAULT_ECE_DRIFT_THRESHOLD = 0.05
DEFAULT_WINDOW_SIZE = 1000
# Mirrors the project-wide sample-size-gate convention (crates/strategy's
# SAMPLE_SIZE_GATE, agents_graph.priors, this package's own backtest
# warm-up): don't judge calibration from too few observations.
DEFAULT_MIN_OBSERVATIONS = 30


@dataclass(frozen=True)
class CalibrationSnapshot:
    n: int
    brier: float
    ece: float
    drifted: bool


class RollingCalibrationMonitor:
    def __init__(
        self,
        window_size: int = DEFAULT_WINDOW_SIZE,
        ece_drift_threshold: float = DEFAULT_ECE_DRIFT_THRESHOLD,
        min_observations: int = DEFAULT_MIN_OBSERVATIONS,
    ) -> None:
        self.window_size = window_size
        self.ece_drift_threshold = ece_drift_threshold
        self.min_observations = min_observations
        self._predicted: deque[float] = deque(maxlen=window_size)
        self._actual: deque[int] = deque(maxlen=window_size)

    def observe(self, predicted_probability: float, realized_outcome: int) -> CalibrationSnapshot:
        """Feeds one new (prediction, outcome) pair into the window — the
        oldest observation is evicted once the window is full — and
        returns the recomputed snapshot."""
        self._predicted.append(predicted_probability)
        self._actual.append(realized_outcome)
        return self.snapshot()

    def observe_many(
        self, predicted_probabilities: NDArray[np.float64], realized_outcomes: NDArray[np.int_]
    ) -> list[CalibrationSnapshot]:
        pairs = zip(predicted_probabilities, realized_outcomes, strict=True)
        return [self.observe(float(p), int(y)) for p, y in pairs]

    def snapshot(self) -> CalibrationSnapshot:
        n = len(self._predicted)
        if n < self.min_observations:
            return CalibrationSnapshot(n=n, brier=0.0, ece=0.0, drifted=False)
        y_true = np.array(self._actual, dtype=np.int64)
        p_pred = np.array(self._predicted, dtype=np.float64)
        brier = brier_score(y_true, p_pred)
        ece = expected_calibration_error(y_true, p_pred)
        drifted = ece > self.ece_drift_threshold
        return CalibrationSnapshot(n=n, brier=brier, ece=ece, drifted=drifted)
