"""Overfitting defenses (§8.7): purged K-fold + embargo, and walk-forward
splitting. Both operate on integer bar indices / period counts, not real
dates — callers map those back to actual timestamps. Real because they're
pure, and because getting either subtly wrong (letting label windows leak
across a fold boundary, or reusing test data) is exactly the "Sharpe of 2.0
after 500 trials is noise" failure mode §8.7 exists to prevent.
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass


@dataclass(frozen=True)
class LabelWindow:
    """A sample's `[start, end]` (inclusive both ends) — the triple-barrier
    entry index and the index at which its label resolved (§8.2). Purging
    needs this whole window, not just the entry index, because a training
    sample "resolves" into the future and can therefore leak information
    about a test period even if the sample itself was taken before that
    period started.
    """

    start: int
    end: int


@dataclass(frozen=True)
class Fold:
    train_indices: list[int]
    test_indices: list[int]


def purged_kfold(label_windows: Sequence[LabelWindow], n_splits: int, embargo: int) -> list[Fold]:
    """§8.7: "Remove samples whose label window overlaps the test fold;
    embargo ~1 label-horizon after each fold."

    Two distinct removals from the training set, both real leakage vectors:
    - **purge**: any training sample whose `[start, end)` window overlaps
      the test fold's own time range at all (its label depended on data
      inside the test period);
    - **embargo**: additionally, samples starting in the `embargo` periods
      immediately *after* the test fold — even a non-overlapping sample
      right after a test period can still carry serial-correlation leakage
      from it.
    """
    n = len(label_windows)
    if n_splits < 2:
        raise ValueError("n_splits must be >= 2")
    fold_edges = [round(i * n / n_splits) for i in range(n_splits + 1)]

    folds = []
    for k in range(n_splits):
        test_start_idx, test_end_idx = fold_edges[k], fold_edges[k + 1]
        test_indices = list(range(test_start_idx, test_end_idx))
        test_time_start = label_windows[test_start_idx].start
        test_time_end = label_windows[test_end_idx - 1].end

        train_indices = []
        for i, w in enumerate(label_windows):
            if test_start_idx <= i < test_end_idx:
                continue
            overlaps_test = w.start <= test_time_end and w.end >= test_time_start
            in_embargo = test_time_end < w.start <= test_time_end + embargo
            if overlaps_test or in_embargo:
                continue
            train_indices.append(i)
        folds.append(Fold(train_indices=train_indices, test_indices=test_indices))
    return folds


@dataclass(frozen=True)
class WalkForwardWindow:
    train_start: int
    train_end: int
    validate_start: int
    validate_end: int
    test_start: int
    test_end: int


def walk_forward_windows(
    total_periods: int, train_periods: int, validate_periods: int, test_periods: int
) -> list[WalkForwardWindow]:
    """§8.7: "Rolling train (12mo) -> validate (2mo) -> test (1mo), step
    forward, never reuse test data." Stepping by exactly `test_periods`
    each iteration is what makes "never reuse test data" true: every
    window's test range is disjoint from every other window's.
    """
    if min(train_periods, validate_periods, test_periods) <= 0:
        raise ValueError("all period lengths must be positive")

    windows = []
    start = 0
    block = train_periods + validate_periods + test_periods
    while start + block <= total_periods:
        train_start = start
        train_end = train_start + train_periods
        validate_start = train_end
        validate_end = validate_start + validate_periods
        test_start = validate_end
        test_end = test_start + test_periods
        windows.append(
            WalkForwardWindow(
                train_start=train_start,
                train_end=train_end,
                validate_start=validate_start,
                validate_end=validate_end,
                test_start=test_start,
                test_end=test_end,
            )
        )
        start += test_periods
    return windows
