"""Automatic outcome resolution (Prompt 7: "automatic outcome resolution
jobs that attach realized results to every prediction"). Two shapes, both
real and OHLC-aware (not close-only):

- `resolve_pattern_outcome`: the same triple-barrier shape as
  `agents_models.labeling.triple_barrier` (§8.2), generalized to
  asymmetric, direction-aware target/invalidation levels — this is §12.3's
  own CONFIRMED/FAILED/TIMEOUT verdict, computed from real subsequent
  price action rather than assumed.
- `resolve_fixed_horizon_move`: §7.2 query #2's "ATR-normalized move at
  +5/+15/+60 min" — a news event has no target/invalidation levels of its
  own, just a fixed horizon to measure against.
"""

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
from enum import StrEnum


class Verdict(StrEnum):
    CONFIRMED = "confirmed"
    FAILED = "failed"
    TIMEOUT = "timeout"


@dataclass(frozen=True)
class OutcomeResolution:
    verdict: Verdict
    bars_to_resolution: int | None  # None on timeout
    mfe: float
    mae: float
    r_multiple: float
    move_pips: float
    move_atr: float
    direction: str  # "Long" | "Short"


def resolve_pattern_outcome(
    *,
    direction: str,
    entry_price: float,
    target_price: float,
    invalidation_price: float,
    subsequent_highs: Sequence[float],
    subsequent_lows: Sequence[float],
    pip_size: float,
    atr: float,
) -> OutcomeResolution:
    if direction not in ("Long", "Short"):
        msg = f"direction must be 'Long' or 'Short', got {direction!r}"
        raise ValueError(msg)
    is_long = direction == "Long"

    risk_distance = abs(entry_price - invalidation_price)
    if risk_distance <= 0:
        msg = "invalidation_price must differ from entry_price"
        raise ValueError(msg)

    max_favorable = 0.0
    max_adverse = 0.0
    for i, (high, low) in enumerate(zip(subsequent_highs, subsequent_lows, strict=True)):
        if is_long:
            max_favorable = max(max_favorable, high - entry_price)
            max_adverse = max(max_adverse, entry_price - low)
            hit_target = high >= target_price
            hit_invalidation = low <= invalidation_price
        else:
            max_favorable = max(max_favorable, entry_price - low)
            max_adverse = max(max_adverse, high - entry_price)
            hit_target = low <= target_price
            hit_invalidation = high >= invalidation_price

        # A bar touching both barriers is resolved conservatively toward
        # the adverse outcome — OHLC bars can't disambiguate intrabar
        # sequencing (the same tick-vs-bar limitation Phase 1 already
        # documents for the bridge/replay path).
        if hit_invalidation:
            return _resolution(
                Verdict.FAILED, i, entry_price, invalidation_price, is_long,
                max_favorable, max_adverse, risk_distance, pip_size, atr,
            )
        if hit_target:
            return _resolution(
                Verdict.CONFIRMED, i, entry_price, target_price, is_long,
                max_favorable, max_adverse, risk_distance, pip_size, atr,
            )

    return OutcomeResolution(
        verdict=Verdict.TIMEOUT,
        bars_to_resolution=None,
        mfe=max_favorable,
        mae=max_adverse,
        r_multiple=0.0,
        move_pips=0.0,
        move_atr=0.0,
        direction=direction,
    )


def _resolution(
    verdict: Verdict,
    bars_to_resolution: int,
    entry_price: float,
    exit_price: float,
    is_long: bool,
    max_favorable: float,
    max_adverse: float,
    risk_distance: float,
    pip_size: float,
    atr: float,
) -> OutcomeResolution:
    signed_move = (exit_price - entry_price) if is_long else (entry_price - exit_price)
    return OutcomeResolution(
        verdict=verdict,
        bars_to_resolution=bars_to_resolution,
        mfe=max_favorable,
        mae=max_adverse,
        r_multiple=signed_move / risk_distance,
        move_pips=signed_move / pip_size,
        move_atr=signed_move / atr if atr > 0 else 0.0,
        direction="Long" if is_long else "Short",
    )


@dataclass(frozen=True)
class FixedHorizonMove:
    move_pips: float
    move_atr: float
    direction: str  # realized direction: "Long" | "Short" | "Flat"
    direction_hit: bool  # did the realized direction match the expectation?


def resolve_fixed_horizon_move(
    *,
    price_at_event: float,
    price_at_horizon: float,
    expected_direction: str,
    pip_size: float,
    atr: float,
) -> FixedHorizonMove:
    """§7.2 query #2: a plain fixed-horizon measurement, no barriers —
    `expected_direction` is what the news agent predicted; the realized
    direction and whether it matches are both computed from the actual
    subsequent price, never assumed."""
    signed_move = price_at_horizon - price_at_event
    if signed_move > 0:
        realized_direction = "Long"
    elif signed_move < 0:
        realized_direction = "Short"
    else:
        realized_direction = "Flat"
    return FixedHorizonMove(
        move_pips=signed_move / pip_size,
        move_atr=signed_move / atr if atr > 0 else 0.0,
        direction=realized_direction,
        direction_hit=realized_direction == expected_direction,
    )
