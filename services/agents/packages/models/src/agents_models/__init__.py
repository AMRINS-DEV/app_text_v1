"""Triple-barrier labeling, meta-labeling, purged K-fold + embargo,
walk-forward, isotonic calibration, Deflated Sharpe, ONNX export (§8.2-8.3,
§15). Phase 3 scope."""

from .labeling import Barrier, triple_barrier

__all__ = ["Barrier", "triple_barrier"]
