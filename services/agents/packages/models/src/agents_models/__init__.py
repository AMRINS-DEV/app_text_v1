"""Triple-barrier labeling, purged K-fold + embargo, walk-forward,
calibration, ONNX export (§8.2-8.3, §8.7, §15)."""

from .calibration import (
    CalibrationBin,
    IsotonicCalibrator,
    brier_score,
    expected_calibration_error,
    max_calibration_gap,
    reliability_diagram,
)
from .cross_validation import (
    Fold,
    LabelWindow,
    WalkForwardWindow,
    purged_kfold,
    walk_forward_windows,
)
from .labeling import Barrier, triple_barrier
from .training import (
    SyntheticDataset,
    TrainConfig,
    export_to_onnx,
    make_synthetic_dataset,
    raw_probability,
    train_gbdt,
)

__all__ = [
    "Barrier",
    "triple_barrier",
    "Fold",
    "LabelWindow",
    "WalkForwardWindow",
    "purged_kfold",
    "walk_forward_windows",
    "CalibrationBin",
    "IsotonicCalibrator",
    "brier_score",
    "expected_calibration_error",
    "max_calibration_gap",
    "reliability_diagram",
    "SyntheticDataset",
    "TrainConfig",
    "export_to_onnx",
    "make_synthetic_dataset",
    "raw_probability",
    "train_gbdt",
]
