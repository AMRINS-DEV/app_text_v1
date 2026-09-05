import pytest
from agents_critic.outcome_tracker import CriticOutcomeTracker


def test_should_reduce_weight_is_false_before_enough_vetoes_resolve():
    tracker = CriticOutcomeTracker(min_resolved_for_judgment=10)
    for i in range(5):
        tracker.record_veto(f"sig-{i}", vetoed_at_ns=i)
        tracker.resolve(f"sig-{i}", would_have_been_profitable=True)
    assert tracker.should_reduce_weight() is False


def test_should_reduce_weight_is_true_once_a_majority_of_resolved_vetoes_would_have_won():
    tracker = CriticOutcomeTracker(min_resolved_for_judgment=10)
    for i in range(10):
        tracker.record_veto(f"sig-{i}", vetoed_at_ns=i)
        tracker.resolve(f"sig-{i}", would_have_been_profitable=(i < 6))  # 6/10 would have won
    assert tracker.should_reduce_weight() is True


def test_should_reduce_weight_is_false_when_most_vetoes_were_correct():
    tracker = CriticOutcomeTracker(min_resolved_for_judgment=10)
    for i in range(10):
        tracker.record_veto(f"sig-{i}", vetoed_at_ns=i)
        tracker.resolve(f"sig-{i}", would_have_been_profitable=(i < 3))  # only 3/10 would have won
    assert tracker.should_reduce_weight() is False


def test_unresolved_vetoes_do_not_count_toward_the_threshold():
    tracker = CriticOutcomeTracker(min_resolved_for_judgment=5)
    for i in range(5):
        tracker.record_veto(f"sig-{i}", vetoed_at_ns=i)
    # None resolved yet.
    assert tracker.resolved_vetoes() == []
    assert tracker.should_reduce_weight() is False


def test_resolving_an_unknown_signal_id_raises():
    tracker = CriticOutcomeTracker()
    with pytest.raises(KeyError):
        tracker.resolve("never-vetoed", would_have_been_profitable=True)
