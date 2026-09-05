import numpy as np
from agents_validation.calibration_monitor import RollingCalibrationMonitor


def test_snapshot_before_min_observations_is_a_placeholder_not_a_false_reading():
    monitor = RollingCalibrationMonitor(min_observations=30)
    for _ in range(29):
        snapshot = monitor.observe(0.9, 0)  # would be badly miscalibrated if it counted
    assert snapshot.n == 29
    assert snapshot.brier == 0.0
    assert snapshot.ece == 0.0
    assert snapshot.drifted is False


def test_a_well_calibrated_stream_does_not_drift():
    # At the package's default window size (1000), a genuinely
    # well-calibrated stream stays under the drift threshold ~99% of the
    # time (empirically verified while tuning these defaults — a smaller
    # window's ECE is dominated by binomial sampling noise even when
    # perfectly calibrated, which would make this test flaky by design).
    rng = np.random.default_rng(1)
    monitor = RollingCalibrationMonitor(min_observations=30)
    snapshot = None
    for _ in range(monitor.window_size):
        p = rng.uniform(0.1, 0.9)
        outcome = 1 if rng.random() < p else 0  # realized frequency matches predicted probability
        snapshot = monitor.observe(p, outcome)
    assert snapshot is not None
    assert snapshot.ece < 0.05
    assert snapshot.drifted is False


def test_a_badly_miscalibrated_stream_drifts():
    monitor = RollingCalibrationMonitor(window_size=200, min_observations=30)
    snapshot = None
    for _ in range(200):
        # Always confident of a win that never happens.
        snapshot = monitor.observe(0.9, 0)
    assert snapshot is not None
    assert snapshot.ece > monitor.ece_drift_threshold
    assert snapshot.drifted is True


def test_the_window_slides_and_forgets_old_observations():
    monitor = RollingCalibrationMonitor(window_size=50, min_observations=30)
    # First 50 observations: well-calibrated (p=0.5, outcomes alternate).
    for i in range(50):
        monitor.observe(0.5, i % 2)
    well_calibrated_snapshot = monitor.snapshot()
    assert well_calibrated_snapshot.drifted is False

    # Push 50 more badly miscalibrated observations -- enough to fully
    # evict the earlier well-calibrated window.
    for _ in range(50):
        drifted_snapshot = monitor.observe(0.95, 0)

    assert drifted_snapshot.n == 50  # window capped, not accumulating forever
    assert drifted_snapshot.drifted is True


def test_observe_many_is_equivalent_to_observing_one_at_a_time():
    predicted = np.array([0.9, 0.9, 0.1, 0.1] * 10)
    actual = np.array([1, 0, 0, 1] * 10)

    a = RollingCalibrationMonitor(window_size=100, min_observations=5)
    snapshots_via_many = a.observe_many(predicted, actual)

    b = RollingCalibrationMonitor(window_size=100, min_observations=5)
    pairs = zip(predicted, actual, strict=True)
    snapshots_one_at_a_time = [b.observe(float(p), int(y)) for p, y in pairs]

    assert snapshots_via_many == snapshots_one_at_a_time


def test_observe_many_rejects_mismatched_lengths():
    monitor = RollingCalibrationMonitor()
    try:
        monitor.observe_many(np.array([0.5, 0.6]), np.array([1]))
        raised = False
    except ValueError:
        raised = True
    assert raised
