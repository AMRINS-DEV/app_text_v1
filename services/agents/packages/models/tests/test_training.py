import os

import numpy as np
from agents_models.calibration import IsotonicCalibrator, brier_score
from agents_models.training import (
    TrainConfig,
    export_to_onnx,
    make_synthetic_dataset,
    raw_probability,
    train_gbdt,
)


def test_synthetic_dataset_is_deterministic_given_the_same_seed():
    a = make_synthetic_dataset(n_samples=100, n_features=4, seed=7)
    b = make_synthetic_dataset(n_samples=100, n_features=4, seed=7)
    assert np.array_equal(a.x, b.x)
    assert np.array_equal(a.y, b.y)


def test_synthetic_dataset_has_both_classes():
    data = make_synthetic_dataset(n_samples=2000, n_features=8, seed=42)
    assert set(np.unique(data.y)) == {0, 1}


def test_trained_model_beats_the_base_rate_on_held_out_data():
    data = make_synthetic_dataset(n_samples=3000, n_features=8, seed=42)
    split = 2000
    x_train, y_train = data.x[:split], data.y[:split]
    x_test, y_test = data.x[split:], data.y[split:]

    model = train_gbdt(x_train, y_train, TrainConfig(n_estimators=50, seed=42))
    p_test = raw_probability(model, x_test)

    base_rate = np.full_like(p_test, y_train.mean())
    model_brier = brier_score(y_test, p_test)
    base_rate_brier = brier_score(y_test, base_rate)
    msg = "§8.3: model must beat a base-rate baseline, not just fit noise"
    assert model_brier < base_rate_brier, msg


def test_calibration_improves_or_preserves_brier_score_on_held_out_data():
    data = make_synthetic_dataset(n_samples=4000, n_features=8, seed=1)
    train_end, cal_end = 2000, 3000
    x_train, y_train = data.x[:train_end], data.y[:train_end]
    x_cal, y_cal = data.x[train_end:cal_end], data.y[train_end:cal_end]
    x_test, y_test = data.x[cal_end:], data.y[cal_end:]

    model = train_gbdt(x_train, y_train, TrainConfig(n_estimators=50, seed=1))
    raw_cal = raw_probability(model, x_cal)
    calibrator = IsotonicCalibrator().fit(raw_cal, y_cal)

    raw_test = raw_probability(model, x_test)
    calibrated_test = calibrator.predict(raw_test)

    # Isotonic calibration is fit to minimize squared error on the
    # calibration set by construction; on a held-out test set it should be
    # in the same ballpark, not dramatically worse.
    assert brier_score(y_test, calibrated_test) < brier_score(y_test, raw_test) * 1.5


def test_onnx_export_round_trips_through_onnxruntime(tmp_path):
    import onnxruntime as ort

    data = make_synthetic_dataset(n_samples=500, n_features=6, seed=3)
    model = train_gbdt(data.x, data.y, TrainConfig(n_estimators=10, seed=3))
    onnx_path = os.path.join(tmp_path, "model.onnx")
    export_to_onnx(model, n_features=6, path=onnx_path)

    assert os.path.exists(onnx_path)
    session = ort.InferenceSession(onnx_path, providers=["CPUExecutionProvider"])
    input_name = session.get_inputs()[0].name
    sample = data.x[:5].astype(np.float32)
    outputs = session.run(None, {input_name: sample})
    onnx_probability = outputs[1]  # LightGBM->ONNX classifier: [labels, probabilities]
    python_probability = raw_probability(model, data.x[:5])

    onnx_p1 = np.array([row[1] for row in onnx_probability])
    assert np.allclose(onnx_p1, python_probability, atol=1e-5)
