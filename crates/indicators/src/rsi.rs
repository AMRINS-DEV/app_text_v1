use crate::Incremental;

/// Wilder's RSI, O(1) per update. Used only regime-gated and as a relative
/// rank rather than an absolute threshold (§8.1) — the strategy VM, not this
/// crate, decides how to use the value.
#[derive(Debug, Clone, Copy)]
pub struct Rsi {
    period: usize,
    prev_price: Option<f64>,
    avg_gain: f64,
    avg_loss: f64,
    count: usize,
    value: Option<f64>,
}

impl Rsi {
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "RSI period must be > 0");
        Self { period, prev_price: None, avg_gain: 0.0, avg_loss: 0.0, count: 0, value: None }
    }
}

impl Incremental for Rsi {
    type Input = f64;
    type Output = Option<f64>;

    fn update(&mut self, price: f64) -> Option<f64> {
        let Some(prev) = self.prev_price else {
            self.prev_price = Some(price);
            return None;
        };
        let change = price - prev;
        let gain = change.max(0.0);
        let loss = (-change).max(0.0);
        self.prev_price = Some(price);

        if self.count < self.period {
            self.avg_gain += gain;
            self.avg_loss += loss;
            self.count += 1;
            if self.count == self.period {
                self.avg_gain /= self.period as f64;
                self.avg_loss /= self.period as f64;
                self.value = Some(Self::rsi_from_averages(self.avg_gain, self.avg_loss));
            }
            return self.value;
        }

        let p = self.period as f64;
        self.avg_gain = (self.avg_gain * (p - 1.0) + gain) / p;
        self.avg_loss = (self.avg_loss * (p - 1.0) + loss) / p;
        self.value = Some(Self::rsi_from_averages(self.avg_gain, self.avg_loss));
        self.value
    }

    fn value(&self) -> Option<f64> {
        self.value
    }
}

impl Rsi {
    fn rsi_from_averages(avg_gain: f64, avg_loss: f64) -> f64 {
        if avg_loss == 0.0 {
            return 100.0;
        }
        let rs = avg_gain / avg_loss;
        100.0 - (100.0 / (1.0 + rs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_between_0_and_100() {
        let mut rsi = Rsi::new(4);
        let prices = [10.0, 10.5, 10.2, 11.0, 10.8, 12.0, 9.0, 15.0];
        for p in prices {
            if let Some(v) = rsi.update(p) {
                assert!((0.0..=100.0).contains(&v), "RSI out of bounds: {v}");
            }
        }
    }

    #[test]
    fn all_gains_saturates_to_100() {
        let mut rsi = Rsi::new(3);
        let mut last = None;
        for p in [1.0, 2.0, 3.0, 4.0, 5.0, 6.0] {
            last = rsi.update(p);
        }
        assert_eq!(last, Some(100.0));
    }
}
