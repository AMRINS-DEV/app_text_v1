import numpy as np
from agents_models.calibration import brier_score
from agents_models.labeling import Barrier
from agents_models.training import TrainConfig, raw_probability, train_gbdt
from agents_validation import generate_labeled_series, y_true_from_labels


def test_series_length_accounts_for_the_max_bars_runway():
    series = generate_labeled_series(n_bars=200, max_bars=40)
    assert len(series.labels) == 200 - 40 - 1
    assert series.prices.shape[0] == len(series.labels)
    assert series.features.shape == (len(series.labels), 8)
    assert series.atr.shape[0] == len(series.labels)


def test_label_windows_match_bars_to_resolution():
    series = generate_labeled_series(n_bars=300, max_bars=40)
    windows_and_bars = zip(series.label_windows, series.bars_to_resolution, strict=True)
    for t0, (window, bars) in enumerate(windows_and_bars):
        assert window.start == t0
        assert window.end == t0 + bars


def test_same_seed_is_fully_deterministic():
    a = generate_labeled_series(n_bars=300, seed=7)
    b = generate_labeled_series(n_bars=300, seed=7)
    assert np.array_equal(a.prices, b.prices)
    assert np.array_equal(a.features, b.features)
    assert a.labels == b.labels


def test_different_seeds_produce_different_series():
    a = generate_labeled_series(n_bars=300, seed=7)
    b = generate_labeled_series(n_bars=300, seed=8)
    assert not np.array_equal(a.prices, b.prices)


def test_labels_are_a_real_mix_not_degenerate():
    series = generate_labeled_series(n_bars=2000, seed=1)
    counts = {label: series.labels.count(label) for label in Barrier}
    # No barrier outcome should be entirely absent -- a degenerate label
    # distribution would make every downstream metric meaningless.
    assert all(count > 0 for count in counts.values())


def test_y_true_from_labels_maps_win_to_one_and_everything_else_to_zero():
    labels = [Barrier.WIN, Barrier.LOSS, Barrier.TIMEOUT, Barrier.WIN]
    assert list(y_true_from_labels(labels)) == [1, 0, 0, 1]


def test_features_carry_real_learnable_signal_above_a_base_rate_guess():
    # Sanity check that the synthetic data is fit for purpose: a GBDT
    # trained on the features should beat a dummy "always guess the base
    # rate" baseline on held-out data -- not by a lot (this is deliberately
    # noisy, per the module's own doc comment), but by a real, measurable
    # margin, proving the signal exists without being unrealistically easy.
    series = generate_labeled_series(n_bars=4000, seed=3)
    y = y_true_from_labels(series.labels)
    split = len(y) * 3 // 4
    x_train, x_test = series.features[:split], series.features[split:]
    y_train, y_test = y[:split], y[split:]

    model = train_gbdt(x_train, y_train, TrainConfig(seed=3))
    p_pred = raw_probability(model, x_test)
    model_brier = brier_score(y_test, p_pred)

    base_rate = y_train.mean()
    baseline_brier = brier_score(y_test, np.full_like(p_pred, base_rate))

    assert model_brier < baseline_brier
