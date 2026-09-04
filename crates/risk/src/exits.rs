//! Exit management (§9.3): stop placement, chandelier trailing, breakeven,
//! time stops. Pure functions operating on fixed-point prices/distances
//! (same scale as `domain::Tick`) — no I/O, no state, so every rule here is
//! independently unit-testable against the doc's own formulas.

use domain::Side;

/// `max(structure_stop, atr_stop, broker_min_stop_level + buffer)` (§9.3) —
/// the *widest* of the three candidate stop distances, so the stop is never
/// placed tighter than the broker allows or the recent structure implies.
/// `structure_distance` is `None` when no structure invalidation level is
/// available (e.g. no swing point yet) and is simply excluded from the max.
pub fn stop_distance(
    atr: f64,
    atr_mult: f64,
    min_pts: f64,
    structure_distance: Option<f64>,
    broker_min_stop_points: f64,
    buffer_points: f64,
) -> f64 {
    let atr_stop = (atr * atr_mult).max(min_pts);
    let broker_min = broker_min_stop_points + buffer_points;
    [atr_stop, broker_min, structure_distance.unwrap_or(0.0)].into_iter().fold(0.0, f64::max)
}

/// Places the stop `distance` fixed-point units on the losing side of `entry`.
pub fn stop_price(entry: i64, side: Side, distance: i64) -> i64 {
    match side {
        Side::Buy => entry - distance,
        Side::Sell => entry + distance,
    }
}

/// Places the target `r_multiple` times the stop distance on the winning
/// side of `entry` (§5.5's `r_multiple` exit type).
pub fn target_price(entry: i64, side: Side, stop_distance: i64, r_multiple: f64) -> i64 {
    let reward_distance = (stop_distance as f64 * r_multiple).round() as i64;
    match side {
        Side::Buy => entry + reward_distance,
        Side::Sell => entry - reward_distance,
    }
}

/// Chandelier exit: `highest_high - atr_mult * atr` for a long position (the
/// mirror, `lowest_low + atr_mult * atr`, for a short). Callers only apply
/// this once the position has reached `activate_at_r` (§9.3) — that
/// threshold isn't this function's concern, it just computes the candidate.
pub fn chandelier_stop(side: Side, extreme_since_entry: i64, atr: f64, atr_mult: f64) -> i64 {
    let offset = (atr * atr_mult).round() as i64;
    match side {
        Side::Buy => extreme_since_entry - offset,
        Side::Sell => extreme_since_entry + offset,
    }
}

/// A trailing stop only ever tightens (moves toward the entry side is
/// never allowed — that would loosen it and give back protected profit).
/// Returns whichever of `current_stop` and `candidate` is more favorable.
pub fn ratchet_stop(side: Side, current_stop: i64, candidate: i64) -> i64 {
    match side {
        Side::Buy => current_stop.max(candidate),
        Side::Sell => current_stop.min(candidate),
    }
}

/// Breakeven move (§9.3): "offset must cover spread + commission, otherwise
/// 'breakeven' is still a loss". `min_cost_buffer` is the exchange's own
/// measured cost (spread + commission, in fixed-point points); the actual
/// offset applied is never less than that, regardless of what the strategy
/// config requested.
pub fn breakeven_stop(entry: i64, side: Side, min_cost_buffer: i64, requested_offset: i64) -> i64 {
    let offset = requested_offset.max(min_cost_buffer);
    match side {
        Side::Buy => entry + offset,
        Side::Sell => entry - offset,
    }
}

/// §9.3/§8.6 item 10: "a thesis that hasn't worked in N bars is invalidated".
pub fn is_time_stop_triggered(bars_held: u32, max_bars: u32) -> bool {
    bars_held >= max_bars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_distance_picks_the_widest_candidate() {
        // atr_stop = 14*1.5=21, broker_min=10+2=12, structure=30 -> widest is structure.
        assert_eq!(stop_distance(14.0, 1.5, 5.0, Some(30.0), 10.0, 2.0), 30.0);
        // Without a structure level, the widest of atr_stop/broker_min wins.
        assert_eq!(stop_distance(14.0, 1.5, 5.0, None, 10.0, 2.0), 21.0);
    }

    #[test]
    fn stop_distance_respects_the_min_pts_floor() {
        // atr*mult = 1*1.5 = 1.5, floored up to min_pts = 30.
        assert_eq!(stop_distance(1.0, 1.5, 30.0, None, 5.0, 1.0), 30.0);
    }

    #[test]
    fn stop_price_is_on_the_losing_side() {
        assert_eq!(stop_price(100_000, Side::Buy, 500), 99_500);
        assert_eq!(stop_price(100_000, Side::Sell, 500), 100_500);
    }

    #[test]
    fn target_price_is_r_multiple_of_stop_distance_on_the_winning_side() {
        assert_eq!(target_price(100_000, Side::Buy, 500, 2.2), 101_100);
        assert_eq!(target_price(100_000, Side::Sell, 500, 2.2), 98_900);
    }

    #[test]
    fn chandelier_trails_below_the_high_for_a_long() {
        assert_eq!(chandelier_stop(Side::Buy, 101_000, 20.0, 2.5), 100_950);
    }

    #[test]
    fn chandelier_trails_above_the_low_for_a_short() {
        assert_eq!(chandelier_stop(Side::Sell, 99_000, 20.0, 2.5), 99_050);
    }

    #[test]
    fn ratchet_never_loosens_a_long_stop() {
        assert_eq!(ratchet_stop(Side::Buy, 100_000, 99_500), 100_000, "candidate is worse, keep current");
        assert_eq!(ratchet_stop(Side::Buy, 100_000, 100_200), 100_200, "candidate is better, tighten");
    }

    #[test]
    fn ratchet_never_loosens_a_short_stop() {
        assert_eq!(ratchet_stop(Side::Sell, 100_000, 100_500), 100_000, "candidate is worse, keep current");
        assert_eq!(ratchet_stop(Side::Sell, 100_000, 99_800), 99_800, "candidate is better, tighten");
    }

    #[test]
    fn breakeven_never_undercuts_real_costs() {
        // Requested offset (5) is below the real cost buffer (12) -> costs win.
        assert_eq!(breakeven_stop(100_000, Side::Buy, 12, 5), 100_012);
        // Requested offset (20) exceeds the cost buffer -> requested wins.
        assert_eq!(breakeven_stop(100_000, Side::Buy, 12, 20), 100_020);
    }

    #[test]
    fn time_stop_triggers_at_exactly_max_bars() {
        assert!(!is_time_stop_triggered(39, 40));
        assert!(is_time_stop_triggered(40, 40));
        assert!(is_time_stop_triggered(41, 40));
    }
}
