//! §9.4: immediate profit-taking at a win margin, implemented *safely* —
//! gated by regime, partial (never a full close by default), and measured
//! against its own counterfactual continuously, because closing whole
//! positions early mechanically lowers `avg_win` and can quietly destroy
//! expectancy even while the equity curve looks smoother (§0.2, §9.4).

use domain::RegimeTag;

#[derive(Debug, Clone)]
pub struct QuickProfitConfig {
    pub enabled: bool,
    /// Fire when unrealized profit reaches this many R.
    pub trigger_r: f64,
    /// Fraction of the position closed when it fires — §9.4 recommends
    /// partial scale-out (0.5) over a full close (1.0) precisely because a
    /// full close at a small trigger_r turns a 2.2R system into a 0.6R one.
    pub close_fraction: f64,
    /// Only fires in these regimes (§9.4: "ONLY in these regimes").
    pub regime_gate: Vec<RegimeTag>,
}

/// Whether the rule fires right now. Pure predicate — the caller still
/// owns actually closing the fraction and moving the remainder to
/// breakeven/trailing (`crates/risk::exits`).
pub fn should_fire(config: &QuickProfitConfig, current_r: f64, regime: RegimeTag) -> bool {
    config.enabled && current_r >= config.trigger_r && config.regime_gate.contains(&regime)
}

/// Splits `total_qty` into (closed, remaining) per `close_fraction`,
/// clamped so it can never close more than the whole position or leave a
/// negative remainder.
pub fn partial_close_quantities(total_qty: i64, close_fraction: f64) -> (i64, i64) {
    let closed = ((total_qty as f64) * close_fraction.clamp(0.0, 1.0)).round() as i64;
    let closed = closed.clamp(0, total_qty);
    (closed, total_qty - closed)
}

/// One resolved trade's outcome under both arms of the §9.4 shadow A/B:
/// what actually happened (quick-profit applied, if it fired) and what
/// would have happened had quick-profit been off throughout (the position
/// held for its ordinary stop/target/trailing exit instead).
#[derive(Debug, Clone, Copy)]
pub struct ShadowOutcome {
    pub with_quick_profit_r: f64,
    pub without_quick_profit_r: f64,
}

/// Continuously measures quick-profit ON vs OFF on the same signals (§9.4):
/// "the dashboard should recommend disabling it" once the delta has been
/// negative for 100+ trades — that threshold is exposed here, not just the
/// running numbers, so the recommendation is a fact this type can assert
/// rather than something a caller has to reimplement.
#[derive(Debug, Default)]
pub struct QuickProfitTracker {
    outcomes: Vec<ShadowOutcome>,
}

impl QuickProfitTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, outcome: ShadowOutcome) {
        self.outcomes.push(outcome);
    }

    pub fn trade_count(&self) -> usize {
        self.outcomes.len()
    }

    pub fn expectancy_with(&self) -> Option<f64> {
        mean(self.outcomes.iter().map(|o| o.with_quick_profit_r))
    }

    pub fn expectancy_without(&self) -> Option<f64> {
        mean(self.outcomes.iter().map(|o| o.without_quick_profit_r))
    }

    /// Positive means quick-profit is helping; negative means it's hurting.
    pub fn expectancy_delta(&self) -> Option<f64> {
        Some(self.expectancy_with()? - self.expectancy_without()?)
    }

    /// §9.4, verbatim threshold: "If the delta is negative for 100+ trades,
    /// the dashboard should recommend disabling it."
    pub fn should_recommend_disabling(&self) -> bool {
        self.trade_count() >= 100 && self.expectancy_delta().is_some_and(|d| d < 0.0)
    }
}

fn mean(values: impl Iterator<Item = f64> + Clone) -> Option<f64> {
    let count = values.clone().count();
    if count == 0 {
        return None;
    }
    Some(values.sum::<f64>() / count as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> QuickProfitConfig {
        QuickProfitConfig {
            enabled: true,
            trigger_r: 0.6,
            close_fraction: 0.5,
            regime_gate: vec![RegimeTag::Ranging, RegimeTag::HighVolChoppy],
        }
    }

    #[test]
    fn fires_only_above_trigger_r_and_inside_the_regime_gate() {
        let cfg = config();
        assert!(!should_fire(&cfg, 0.5, RegimeTag::Ranging), "below trigger_r");
        assert!(should_fire(&cfg, 0.6, RegimeTag::Ranging), "at trigger_r, gated regime");
        assert!(!should_fire(&cfg, 0.9, RegimeTag::Trending), "above trigger_r but wrong regime");
    }

    #[test]
    fn disabled_config_never_fires_even_if_conditions_are_met() {
        let mut cfg = config();
        cfg.enabled = false;
        assert!(!should_fire(&cfg, 5.0, RegimeTag::Ranging));
    }

    #[test]
    fn partial_close_splits_correctly_and_never_overshoots() {
        assert_eq!(partial_close_quantities(100, 0.5), (50, 50));
        assert_eq!(partial_close_quantities(3, 0.5), (2, 1)); // rounds, doesn't lose units
        assert_eq!(partial_close_quantities(10, 1.5), (10, 0), "fraction > 1 clamps to a full close");
        assert_eq!(partial_close_quantities(10, -0.5), (0, 10), "negative fraction clamps to no close");
    }

    #[test]
    fn tracker_reports_none_with_no_trades() {
        let tracker = QuickProfitTracker::new();
        assert_eq!(tracker.expectancy_delta(), None);
        assert!(!tracker.should_recommend_disabling());
    }

    #[test]
    fn tracker_computes_the_expectancy_delta() {
        let mut tracker = QuickProfitTracker::new();
        tracker.record(ShadowOutcome { with_quick_profit_r: 0.6, without_quick_profit_r: 2.2 });
        tracker.record(ShadowOutcome { with_quick_profit_r: 0.6, without_quick_profit_r: -1.0 });
        // with: mean(0.6,0.6)=0.6 ; without: mean(2.2,-1.0)=0.6 -> delta 0
        assert!((tracker.expectancy_delta().unwrap()).abs() < 1e-9);
    }

    #[test]
    fn recommends_disabling_only_after_100_trades_of_negative_delta() {
        let mut tracker = QuickProfitTracker::new();
        for _ in 0..99 {
            tracker.record(ShadowOutcome { with_quick_profit_r: 0.6, without_quick_profit_r: 2.2 });
        }
        assert!(!tracker.should_recommend_disabling(), "only 99 trades so far");
        tracker.record(ShadowOutcome { with_quick_profit_r: 0.6, without_quick_profit_r: 2.2 });
        assert!(tracker.should_recommend_disabling(), "100 trades, consistently negative delta");
    }

    #[test]
    fn does_not_recommend_disabling_when_quick_profit_is_net_positive() {
        let mut tracker = QuickProfitTracker::new();
        for _ in 0..150 {
            tracker.record(ShadowOutcome { with_quick_profit_r: 0.6, without_quick_profit_r: 0.1 });
        }
        assert!(!tracker.should_recommend_disabling());
    }
}
