import pytest
from agents_models.cross_validation import LabelWindow, purged_kfold, walk_forward_windows


def make_windows(n: int, horizon: int) -> list[LabelWindow]:
    # Sample i starts at bar i, resolves `horizon` bars later (like a
    # constant-horizon triple-barrier label).
    return [LabelWindow(start=i, end=i + horizon) for i in range(n)]


def test_purged_kfold_produces_disjoint_test_folds_covering_everything():
    windows = make_windows(20, horizon=1)
    folds = purged_kfold(windows, n_splits=4, embargo=0)
    all_test = sorted(idx for fold in folds for idx in fold.test_indices)
    assert all_test == list(range(20))


def test_purged_kfold_removes_overlapping_training_samples():
    # horizon=5 means sample i's label resolves at i+5, so samples just
    # before a test fold overlap into it and must be purged.
    windows = make_windows(20, horizon=5)
    folds = purged_kfold(windows, n_splits=4, embargo=0)
    # Fold 1 is indices [5,10). Sample 4's window [4,9] overlaps into it.
    fold1 = folds[1]
    msg = "a training sample whose label window overlaps the test fold must be purged"
    assert 4 not in fold1.train_indices, msg


def test_purged_kfold_embargo_removes_samples_right_after_the_test_fold():
    windows = make_windows(20, horizon=1)  # window [i, i+1]; fold 0 -> test_time_end = 5
    folds_no_embargo = purged_kfold(windows, n_splits=4, embargo=0)
    folds_with_embargo = purged_kfold(windows, n_splits=4, embargo=3)
    # Sample 6's window [6,7] does not overlap fold 0's test range [0,5] at
    # all, so with no embargo it's a valid training sample...
    assert 6 in folds_no_embargo[0].train_indices
    # ...but a 3-period embargo after test_time_end=5 covers (5,8], which includes 6.
    msg = "embargo must remove samples starting shortly after the test fold"
    assert 6 not in folds_with_embargo[0].train_indices, msg


def test_purged_kfold_rejects_fewer_than_two_splits():
    with pytest.raises(ValueError):
        purged_kfold(make_windows(10, 1), n_splits=1, embargo=0)


def test_walk_forward_windows_step_by_exactly_the_test_period():
    windows = walk_forward_windows(
        total_periods=100, train_periods=60, validate_periods=10, test_periods=10
    )
    assert len(windows) >= 2
    first, second = windows[0], windows[1]
    assert second.train_start == first.train_start + 10
    assert second.test_start == first.test_start + 10


def test_walk_forward_windows_never_reuse_test_data():
    windows = walk_forward_windows(
        total_periods=100, train_periods=60, validate_periods=10, test_periods=10
    )
    test_ranges = [set(range(w.test_start, w.test_end)) for w in windows]
    for i, a in enumerate(test_ranges):
        for b in test_ranges[i + 1 :]:
            assert a.isdisjoint(b), "no two windows may reuse the same test period"


def test_walk_forward_windows_respects_ordering_train_before_validate_before_test():
    windows = walk_forward_windows(
        total_periods=200, train_periods=60, validate_periods=10, test_periods=10
    )
    for w in windows:
        assert w.train_start < w.train_end == w.validate_start
        assert w.validate_start < w.validate_end == w.test_start
        assert w.test_start < w.test_end


def test_walk_forward_windows_rejects_non_positive_periods():
    with pytest.raises(ValueError):
        walk_forward_windows(
            total_periods=100, train_periods=0, validate_periods=10, test_periods=10
        )
