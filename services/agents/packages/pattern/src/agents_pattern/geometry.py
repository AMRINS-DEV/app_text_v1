"""Deterministic pattern detection over OHLCV arrays (§10.1: "mostly
deterministic code; LLM only for narrative"). Double-top/double-bottom
only — the doc's full pattern library (head-and-shoulders, triangles,
flags, etc.) is future work; this establishes the real geometry-plus-
target-plus-invalidation shape every other pattern detector would follow.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

import numpy as np


@dataclass(frozen=True)
class SwingPoint:
    index: int
    price: float
    kind: Literal["high", "low"]


def find_swing_points(highs: np.ndarray, lows: np.ndarray, window: int = 3) -> list[SwingPoint]:
    """A bar is a swing high/low if it is the strict extreme among the
    `window` bars on each side of it — the same definition
    `crates/indicators::swing` uses on the Rust side, reimplemented here
    rather than shared cross-language (Python and Rust each operate on
    their own OHLCV representation in this repo)."""
    points: list[SwingPoint] = []
    n = len(highs)
    for i in range(window, n - window):
        left_highs, right_highs = highs[i - window : i], highs[i + 1 : i + window + 1]
        if highs[i] > left_highs.max() and highs[i] > right_highs.max():
            points.append(SwingPoint(index=i, price=float(highs[i]), kind="high"))
        left_lows, right_lows = lows[i - window : i], lows[i + 1 : i + window + 1]
        if lows[i] < left_lows.min() and lows[i] < right_lows.min():
            points.append(SwingPoint(index=i, price=float(lows[i]), kind="low"))
    return points


@dataclass(frozen=True)
class PatternInstance:
    kind: Literal["double_top", "double_bottom"]
    peak_indices: tuple[int, int]
    peak_price: float
    neckline_price: float
    target_price: float
    invalidation_price: float
    confidence: float


def detect_double_top_or_bottom(
    highs: np.ndarray,
    lows: np.ndarray,
    window: int = 3,
    tolerance_pct: float = 0.005,
) -> PatternInstance | None:
    """A double top/bottom: two swing extrema of the same kind, within
    `tolerance_pct` of each other's price, with an opposite-kind swing
    point (the "neckline") in between. Target = neckline -/+ the
    peak-to-neckline distance (the standard "measured move"); invalidation
    = a close back beyond the peaks. Returns the *most recent* such
    pattern, or `None` if none is found — "no pattern" is a real, common
    outcome, not an error."""
    swings = find_swing_points(highs, lows, window=window)
    highs_list = [s for s in swings if s.kind == "high"]
    lows_list = [s for s in swings if s.kind == "low"]

    best: PatternInstance | None = None

    for kind, extrema, neckline_extrema, target_below in (
        ("double_top", highs_list, lows_list, True),
        ("double_bottom", lows_list, highs_list, False),
    ):
        for i in range(len(extrema) - 1):
            a, b = extrema[i], extrema[i + 1]
            if abs(a.price - b.price) / max(abs(a.price), 1e-12) > tolerance_pct:
                continue
            between = [p for p in neckline_extrema if a.index < p.index < b.index]
            if not between:
                continue
            neckline = min(between, key=lambda p: p.price) if target_below else max(
                between, key=lambda p: p.price
            )
            peak_price = (a.price + b.price) / 2
            distance = abs(peak_price - neckline.price)
            target = neckline.price - distance if target_below else neckline.price + distance
            invalidation = max(a.price, b.price) if target_below else min(a.price, b.price)
            peak_gap_pct = abs(a.price - b.price) / max(abs(a.price), 1e-12)
            confidence = max(0.0, 1.0 - peak_gap_pct / tolerance_pct)

            candidate = PatternInstance(
                kind=kind,  # type: ignore[arg-type]
                peak_indices=(a.index, b.index),
                peak_price=peak_price,
                neckline_price=neckline.price,
                target_price=target,
                invalidation_price=invalidation,
                confidence=confidence,
            )
            if best is None or b.index > best.peak_indices[1]:
                best = candidate

    return best
