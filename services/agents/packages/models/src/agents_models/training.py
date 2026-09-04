"""GBDT training + ONNX export (§3.2: "Gradient boosting is the workhorse
for tabular market features"; §15: reproducible training). Deep nets for
sequence/vision (PyTorch, per the design doc's stack table) are out of
scope here — this crate covers the tabular GBDT path the doc calls the
workhorse, not every model family it lists.

The dataset used throughout this module is synthetic and deterministic
(fixed seed, no real market data — this environment has none) — its only
job is to exercise the training/calibration/export pipeline end-to-end
with something that actually has *some* learnable signal, so calibration
metrics are meaningful rather than degenerate. It is not a trading model.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import cast

import lightgbm as lgb
import numpy as np
from numpy.typing import NDArray


@dataclass(frozen=True)
class SyntheticDataset:
    x: NDArray[np.float64]
    y: NDArray[np.int_]


def make_synthetic_dataset(
    n_samples: int = 2000, n_features: int = 8, seed: int = 42
) -> SyntheticDataset:
    """A logistic function of a linear combination of features, plus noise
    — enough real signal that a GBDT can learn something above chance,
    enough noise that it can't reach the same "80%+ accuracy" trap §0.1
    warns is "either overfit, look-ahead biased, or measuring on a
    non-tradable horizon".
    """
    rng = np.random.default_rng(seed)
    x = rng.normal(size=(n_samples, n_features))
    true_weights = rng.normal(size=n_features) * 0.5
    # Noise dominates the signal on purpose (see the module docstring).
    logits = x @ true_weights + rng.normal(scale=1.5, size=n_samples)
    probability = 1.0 / (1.0 + np.exp(-logits))
    y = (rng.uniform(size=n_samples) < probability).astype(np.int64)
    return SyntheticDataset(x=x, y=y)


@dataclass(frozen=True)
class TrainConfig:
    n_estimators: int = 50
    num_leaves: int = 7
    learning_rate: float = 0.1
    seed: int = 42


def train_gbdt(
    x_train: NDArray[np.float64], y_train: NDArray[np.int_], config: TrainConfig | None = None
) -> lgb.LGBMClassifier:
    config = config or TrainConfig()
    model = lgb.LGBMClassifier(
        n_estimators=config.n_estimators,
        num_leaves=config.num_leaves,
        learning_rate=config.learning_rate,
        random_state=config.seed,
        deterministic=True,
        verbosity=-1,
    )
    model.fit(x_train, y_train)
    return model


def raw_probability(model: lgb.LGBMClassifier, x: NDArray[np.float64]) -> NDArray[np.float64]:
    """The model's own uncalibrated P(class=1) — §8.3: never used directly
    for sizing, only as isotonic regression's input.
    """
    probabilities = cast(NDArray[np.float64], model.predict_proba(x))
    return probabilities[:, 1]


def export_to_onnx(model: lgb.LGBMClassifier, n_features: int, path: str) -> None:
    """§3.2/§17 Phase 3: "ONNX export with a parity test asserting Rust
    `ort` inference matches Python within 1e-6" (that parity test itself
    lives in `crates/strategy`, Rust side — this function only produces the
    artifact it tests against).
    """
    from onnxmltools import convert_lightgbm
    from onnxmltools.convert.common.data_types import FloatTensorType

    initial_types = [("input", FloatTensorType([None, n_features]))]
    onnx_model = convert_lightgbm(model, initial_types=initial_types)
    with open(path, "wb") as f:
        f.write(onnx_model.SerializeToString())
