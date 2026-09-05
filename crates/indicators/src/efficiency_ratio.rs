use std::collections::VecDeque;

use crate::Incremental;

/// Kaufman's Efficiency Ratio (§8.1): `|price_change over period| /
/// sum(|each bar's change|)` — 1.0 means the price moved in a straight
/// line (trend), near 0 means it churned in place (noise/range). One of
/// the "robust out-of-sample lift" multi-timeframe/trend features §8.1
/// calls out.
pub struct EfficiencyRatio {
    period: usize,
    prices: VecDeque<f64>,
    abs_changes: VecDeque<f64>,
    abs_change_sum: f64,
}

impl EfficiencyRatio {
    pub fn new(period: usize) -> Self {
        assert!(period > 0);
        Self {
            period,
            prices: VecDeque::with_capacity(period + 1),
            abs_changes: VecDeque::with_capacity(period),
            abs_change_sum: 0.0,
        }
    }
}

impl Incremental for EfficiencyRatio {
    type Input = f64;
    type Output = Option<f64>;

    fn update(&mut self, price: f64) -> Self::Output {
        if let Some(&prev) = self.prices.back() {
            let change = (price - prev).abs();
            self.abs_changes.push_back(change);
            self.abs_change_sum += change;
            if self.abs_changes.len() > self.period {
                if let Some(evicted) = self.abs_changes.pop_front() {
                    self.abs_change_sum -= evicted;
                }
            }
        }
        self.prices.push_back(price);
        if self.prices.len() > self.period + 1 {
            self.prices.pop_front();
        }
        self.value()
    }

    fn value(&self) -> Self::Output {
        if self.abs_changes.len() < self.period {
            return None;
        }
        let net_change = (self.prices.back()? - self.prices.front()?).abs();
        if self.abs_change_sum == 0.0 {
            return Some(0.0);
        }
        Some(net_change / self.abs_change_sum)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_straight_line_trend_has_efficiency_ratio_of_one() {
        let mut er = EfficiencyRatio::new(4);
        let mut last = None;
        for p in [10.0, 11.0, 12.0, 13.0, 14.0] {
            last = er.update(p);
        }
        assert!((last.unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_perfectly_oscillating_series_has_efficiency_ratio_near_zero() {
        let mut er = EfficiencyRatio::new(4);
        let mut last = None;
        for p in [10.0, 11.0, 10.0, 11.0, 10.0] {
            last = er.update(p);
        }
        assert!(last.unwrap().abs() < 1e-9);
    }

    #[test]
    fn stays_bounded_between_zero_and_one() {
        let mut er = EfficiencyRatio::new(3);
        for p in [10.0, 15.0, 9.0, 20.0, 3.0, 11.0] {
            if let Some(v) = er.update(p) {
                assert!((0.0..=1.0).contains(&v));
            }
        }
    }
}
