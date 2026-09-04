use crate::Incremental;

#[derive(Debug, Clone, Copy)]
pub struct OhlcInput {
    pub high: f64,
    pub low: f64,
    pub close: f64,
}

/// Wilder's Average True Range, O(1) per bar. Every stop/target/threshold
/// elsewhere in the system is normalized by this (§8.1: "every threshold
/// must be volatility-normalized or the model breaks when regime shifts").
#[derive(Debug, Clone, Copy)]
pub struct Atr {
    period: usize,
    prev_close: Option<f64>,
    value: Option<f64>,
    seed_sum: f64,
    seed_count: usize,
}

impl Atr {
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "ATR period must be > 0");
        Self { period, prev_close: None, value: None, seed_sum: 0.0, seed_count: 0 }
    }

    fn true_range(&self, bar: OhlcInput) -> f64 {
        let hl = bar.high - bar.low;
        match self.prev_close {
            None => hl,
            Some(pc) => hl.max((bar.high - pc).abs()).max((bar.low - pc).abs()),
        }
    }
}

impl Incremental for Atr {
    type Input = OhlcInput;
    type Output = Option<f64>;

    fn update(&mut self, bar: OhlcInput) -> Option<f64> {
        let tr = self.true_range(bar);
        self.value = Some(match self.value {
            None => {
                // Seed with a simple average over the first `period` true ranges,
                // as Wilder's method prescribes, then switch to the smoothed form.
                self.seed_sum += tr;
                self.seed_count += 1;
                if self.seed_count < self.period {
                    self.prev_close = Some(bar.close);
                    return None;
                }
                self.seed_sum / self.period as f64
            }
            Some(prev) => (prev * (self.period as f64 - 1.0) + tr) / self.period as f64,
        });
        self.prev_close = Some(bar.close);
        self.value
    }

    fn value(&self) -> Option<f64> {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warms_up_before_emitting() {
        let mut atr = Atr::new(3);
        assert_eq!(atr.update(OhlcInput { high: 10.0, low: 9.0, close: 9.5 }), None);
        assert_eq!(atr.update(OhlcInput { high: 10.5, low: 9.5, close: 10.0 }), None);
        assert!(atr.update(OhlcInput { high: 11.0, low: 10.0, close: 10.5 }).is_some());
    }

    #[test]
    fn never_negative() {
        let mut atr = Atr::new(2);
        atr.update(OhlcInput { high: 10.0, low: 9.0, close: 9.5 });
        let v = atr.update(OhlcInput { high: 10.2, low: 9.8, close: 10.0 }).unwrap();
        assert!(v >= 0.0);
    }
}
