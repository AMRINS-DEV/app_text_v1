import numpy as np
from agents_models.calibration import (
    IsotonicCalibrator,
    brier_score,
    expected_calibration_error,
    max_calibration_gap,
    reliability_diagram,
)


def test_brier_score_is_zero_for_perfect_predictions():
    y_true = np.array([1, 0, 1, 0])
    p_pred = np.array([1.0, 0.0, 1.0, 0.0])
    assert brier_score(y_true, p_pred) == 0.0


def test_brier_score_is_worse_for_confidently_wrong_predictions():
    y_true = np.array([1, 0])
    good = brier_score(y_true, np.array([0.9, 0.1]))
    bad = brier_score(y_true, np.array([0.1, 0.9]))
    assert bad > good


def test_reliability_diagram_is_perfect_for_well_calibrated_predictions():
    rng = np.random.default_rng(0)
    p_pred = rng.uniform(size=5000)
    y_true = (rng.uniform(size=5000) < p_pred).astype(int)
    bins = reliability_diagram(y_true, p_pred, n_bins=10)
    assert len(bins) == 10
    for b in bins:
        assert abs(b.mean_predicted - b.empirical_frequency) < 0.05


def test_ece_is_near_zero_for_well_calibrated_predictions():
    rng = np.random.default_rng(1)
    p_pred = rng.uniform(size=5000)
    y_true = (rng.uniform(size=5000) < p_pred).astype(int)
    assert expected_calibration_error(y_true, p_pred) < 0.03, "§8.3 ECE target"


def test_ece_is_large_for_badly_calibrated_predictions():
    # Predicts 0.9 for everything regardless of true outcome frequency (~0.3).
    rng = np.random.default_rng(2)
    y_true = (rng.uniform(size=2000) < 0.3).astype(int)
    p_pred = np.full(2000, 0.9)
    assert expected_calibration_error(y_true, p_pred) > 0.3


def test_max_calibration_gap_flags_a_single_bad_bucket_that_ece_averages_away():
    # Nine buckets perfectly calibrated, one badly miscalibrated -> ECE
    # (averaged) stays small but max_calibration_gap (§8.3's >5pp rule) does not.
    rng = np.random.default_rng(3)
    n_good = 9000
    p_good = rng.uniform(size=n_good)
    y_good = (rng.uniform(size=n_good) < p_good).astype(int)
    p_bad = np.full(1000, 0.95)
    y_bad = (rng.uniform(size=1000) < 0.5).astype(int)  # true frequency ~0.5, predicted 0.95
    p_pred = np.concatenate([p_good, p_bad])
    y_true = np.concatenate([y_good, y_bad])
    assert max_calibration_gap(y_true, p_pred) > 0.05
    assert expected_calibration_error(y_true, p_pred) < max_calibration_gap(y_true, p_pred)


def test_isotonic_calibrator_output_is_clamped_away_from_zero_and_one():
    calibrator = IsotonicCalibrator(eps=1e-4)
    raw = np.array([0.0, 0.2, 0.5, 0.8, 1.0])
    y = np.array([0, 0, 1, 1, 1])
    calibrator.fit(raw, y)
    calibrated = calibrator.predict(raw)
    assert np.all(calibrated > 0.0)
    assert np.all(calibrated < 1.0)


def test_isotonic_calibrator_predict_before_fit_raises():
    calibrator = IsotonicCalibrator()
    try:
        calibrator.predict(np.array([0.5]))
        raise AssertionError("expected RuntimeError")
    except RuntimeError:
        pass
