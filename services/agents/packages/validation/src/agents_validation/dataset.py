"""A synthetic labeled time series connecting `agents_models.labeling`'s
`triple_barrier`, `agents_models.cross_validation`'s `LabelWindow`, and
`agents_models.training`'s GBDT for the first time — those three modules
are pure, tested ingredients with nothing in Phase 3 orchestrating them
together over an actual time-ordered price series. Closing that gap is
this module's whole job.

Deterministic and synthetic (no real market data exists in this sandbox),
same "real signal, dominant noise" discipline as `training.py`'s own
dataset — with one addition `training.py` doesn't need: the drift has to
be *persistent* across several bars, not a single-step nudge, because a
triple-barrier label resolves over up to `max_bars` future bars. A feature
that only predicts the very next bar's return gets completely swamped by
the other `max_bars - 1` bars of pure noise on the way to a barrier —
verified empirically while building this (an i.i.d.-drift version of this
generator produced no measurable signal at all). The fix is a slowly
mean-reverting latent "regime" (an AR(1) process) that drives returns over
many consecutive bars; `features[:, 0]` is a noisy readout of that same
regime (features 1..n are pure decoys), so a value observed at bar `t0`
carries information about the *whole* window ahead of it, not just the
next tick.
"""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np
from agents_models.cross_validation import LabelWindow
from agents_models.labeling import Barrier, triple_barrier
from numpy.typing import NDArray

_ATR_WINDOW = 14
# The raw mean-absolute-return ATR proxy is scaled up before use as a
# barrier distance: with i.i.d.-ish returns, reaching `tp_mult`/`sl_mult`
# multiples of a *single bar's* typical move takes only 2-5 bars via
# diffusion alone (crossing time ~ (distance / per-bar std)^2), which
# makes `max_bars=40` a ceiling that's never binding and TIMEOUT
# essentially impossible — confirmed empirically before adding this
# factor (0% timeouts at any noise level tried). A real ATR computed from
# genuine OHLC ranges over a longer natural bar period wouldn't need this
# correction; this synthetic series only has closes, so the proxy needs
# it to reach a realistic multi-bar resolution horizon.
_ATR_SCALE = 4.0
_REGIME_AR_COEFFICIENT = 0.97
_REGIME_NOISE_STD = 0.15
_DRIFT_SCALE = 0.0008
_RETURN_NOISE_STD = 0.0015
_FEATURE_NOISE_STD = 0.5


@dataclass(frozen=True)
class LabeledSeries:
    prices: NDArray[np.float64]
    features: NDArray[np.float64]  # shape (n_usable_bars, n_features)
    atr: NDArray[np.float64]
    labels: list[Barrier]
    bars_to_resolution: list[int]
    label_windows: list[LabelWindow]


def y_true_from_labels(labels: list[Barrier]) -> NDArray[np.int_]:
    """§8.2's primary bet is always long (`triple_barrier` only tests an
    upper/lower barrier pair, no direction parameter) — WIN is the only
    "the bet paid off" outcome; LOSS and TIMEOUT both count as 0."""
    return np.array([1 if label == Barrier.WIN else 0 for label in labels], dtype=np.int64)


def _rolling_atr_proxy(prices: NDArray[np.float64], window: int) -> NDArray[np.float64]:
    """A simplified ATR proxy — mean absolute bar-to-bar return over a
    trailing window. Real formula, simplified because this synthetic
    series only has close prices, not real OHLC ranges."""
    abs_returns = np.abs(np.diff(prices, prepend=prices[0]))
    atr = np.empty_like(prices)
    for i in range(len(prices)):
        lo = max(0, i - window + 1)
        atr[i] = abs_returns[lo : i + 1].mean()
    return np.maximum(atr, 1e-8)


def generate_labeled_series(
    n_bars: int = 4000,
    n_features: int = 8,
    seed: int = 42,
    tp_mult: float = 2.2,
    sl_mult: float = 1.5,
    max_bars: int = 40,
) -> LabeledSeries:
    rng = np.random.default_rng(seed)

    regime = np.zeros(n_bars)
    for i in range(1, n_bars):
        regime[i] = _REGIME_AR_COEFFICIENT * regime[i - 1] + rng.normal(scale=_REGIME_NOISE_STD)

    returns = regime * _DRIFT_SCALE + rng.normal(scale=_RETURN_NOISE_STD, size=n_bars)
    prices = 100.0 * np.exp(np.cumsum(returns))
    atr = _rolling_atr_proxy(prices, _ATR_WINDOW) * _ATR_SCALE

    features = rng.normal(size=(n_bars, n_features))
    features[:, 0] = regime + rng.normal(scale=_FEATURE_NOISE_STD, size=n_bars)

    # Only bars with a full `max_bars` runway ahead can be labeled.
    usable = max(0, n_bars - max_bars - 1)
    prices_list = prices.tolist()

    labels: list[Barrier] = []
    bars_to_resolution: list[int] = []
    label_windows: list[LabelWindow] = []
    for t0 in range(usable):
        result = triple_barrier(prices_list, t0, float(atr[t0]), tp_mult, sl_mult, max_bars)
        labels.append(result.label)
        bars_to_resolution.append(result.bars_to_resolution)
        label_windows.append(LabelWindow(start=t0, end=t0 + result.bars_to_resolution))

    return LabeledSeries(
        prices=prices[:usable],
        features=features[:usable],
        atr=atr[:usable],
        labels=labels,
        bars_to_resolution=bars_to_resolution,
        label_windows=label_windows,
    )
