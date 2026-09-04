use crate::atr::OhlcInput;
use crate::Incremental;

/// Wilder's Average Directional Index (§8.1 "Trend/regime": ADX — "the
/// single biggest determinant of whether mean-reversion or momentum logic
/// applies"). Two-stage Wilder smoothing: +DM/-DM/TR are smoothed to get
/// +DI/-DI/DX per bar, then DX itself is smoothed to get ADX — so this
/// warms up over roughly `2 * period` bars, not `period`.
pub struct Adx {
    period: usize,
    prev_bar: Option<OhlcInput>,
    smoothed_plus_dm: Option<f64>,
    smoothed_minus_dm: Option<f64>,
    smoothed_tr: Option<f64>,
    seed_sum_plus_dm: f64,
    seed_sum_minus_dm: f64,
    seed_sum_tr: f64,
    seed_count: usize,
    adx: Option<f64>,
    adx_seed_sum: f64,
    adx_seed_count: usize,
}

impl Adx {
    pub fn new(period: usize) -> Self {
        assert!(period > 0);
        Self {
            period,
            prev_bar: None,
            smoothed_plus_dm: None,
            smoothed_minus_dm: None,
            smoothed_tr: None,
            seed_sum_plus_dm: 0.0,
            seed_sum_minus_dm: 0.0,
            seed_sum_tr: 0.0,
            seed_count: 0,
            adx: None,
            adx_seed_sum: 0.0,
            adx_seed_count: 0,
        }
    }

    fn true_range(prev: &OhlcInput, bar: &OhlcInput) -> f64 {
        (bar.high - bar.low).max((bar.high - prev.close).abs()).max((bar.low - prev.close).abs())
    }

    fn wilder_smooth(prev_smoothed: f64, period: usize, current: f64) -> f64 {
        (prev_smoothed * (period as f64 - 1.0) + current) / period as f64
    }
}

impl Incremental for Adx {
    type Input = OhlcInput;
    type Output = Option<f64>;

    fn update(&mut self, bar: OhlcInput) -> Self::Output {
        let Some(prev) = self.prev_bar else {
            self.prev_bar = Some(bar);
            return None;
        };

        let up_move = bar.high - prev.high;
        let down_move = prev.low - bar.low;
        let plus_dm = if up_move > down_move && up_move > 0.0 { up_move } else { 0.0 };
        let minus_dm = if down_move > up_move && down_move > 0.0 { down_move } else { 0.0 };
        let tr = Self::true_range(&prev, &bar);
        self.prev_bar = Some(bar);

        // Stage 1: Wilder-smooth +DM/-DM/TR (same seeding pattern as `Atr`).
        let (plus_dm_s, minus_dm_s, tr_s) = match (self.smoothed_plus_dm, self.smoothed_minus_dm, self.smoothed_tr) {
            (Some(p), Some(m), Some(t)) => (
                Self::wilder_smooth(p, self.period, plus_dm),
                Self::wilder_smooth(m, self.period, minus_dm),
                Self::wilder_smooth(t, self.period, tr),
            ),
            _ => {
                self.seed_sum_plus_dm += plus_dm;
                self.seed_sum_minus_dm += minus_dm;
                self.seed_sum_tr += tr;
                self.seed_count += 1;
                if self.seed_count < self.period {
                    return None;
                }
                (self.seed_sum_plus_dm / self.period as f64, self.seed_sum_minus_dm / self.period as f64, self.seed_sum_tr / self.period as f64)
            }
        };
        self.smoothed_plus_dm = Some(plus_dm_s);
        self.smoothed_minus_dm = Some(minus_dm_s);
        self.smoothed_tr = Some(tr_s);

        // Stage 2: DX from the smoothed DI values, then Wilder-smooth DX itself into ADX.
        let (plus_di, minus_di) = if tr_s > 0.0 { (100.0 * plus_dm_s / tr_s, 100.0 * minus_dm_s / tr_s) } else { (0.0, 0.0) };
        let di_sum = plus_di + minus_di;
        let dx = if di_sum > 0.0 { 100.0 * (plus_di - minus_di).abs() / di_sum } else { 0.0 };

        match self.adx {
            Some(prev_adx) => {
                self.adx = Some(Self::wilder_smooth(prev_adx, self.period, dx));
            }
            None => {
                self.adx_seed_sum += dx;
                self.adx_seed_count += 1;
                if self.adx_seed_count >= self.period {
                    self.adx = Some(self.adx_seed_sum / self.period as f64);
                }
            }
        }
        self.adx
    }

    fn value(&self) -> Self::Output {
        self.adx
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(high: f64, low: f64, close: f64) -> OhlcInput {
        OhlcInput { high, low, close }
    }

    #[test]
    fn warms_up_over_roughly_2x_period_before_emitting() {
        let mut adx = Adx::new(3);
        let mut last = None;
        for i in 0..5 {
            let base = 100.0 + i as f64;
            last = adx.update(bar(base + 1.0, base - 1.0, base));
        }
        assert_eq!(last, None, "should still be warming up after only period+2 bars");
    }

    #[test]
    fn a_strong_steady_uptrend_produces_a_high_adx() {
        let mut adx = Adx::new(3);
        let mut last = None;
        for i in 0..20 {
            let base = 100.0 + i as f64 * 2.0; // steady, strong directional move
            last = adx.update(bar(base + 1.0, base - 1.0, base));
        }
        assert!(last.is_some());
        assert!(last.unwrap() > 50.0, "a clean steady trend should show high ADX, got {last:?}");
    }

    #[test]
    fn stays_within_0_to_100() {
        let mut adx = Adx::new(3);
        let prices = [100.0, 102.0, 99.0, 105.0, 98.0, 110.0, 95.0, 108.0, 101.0, 112.0, 90.0, 115.0];
        for &p in &prices {
            if let Some(v) = adx.update(bar(p + 1.0, p - 1.0, p)) {
                assert!((0.0..=100.0).contains(&v), "ADX out of bounds: {v}");
            }
        }
    }
}
