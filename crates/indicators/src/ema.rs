use crate::Incremental;

/// Exponential moving average, O(1) per update.
#[derive(Debug, Clone, Copy)]
pub struct Ema {
    alpha: f64,
    value: Option<f64>,
}

impl Ema {
    /// `period` in bars, using the standard `alpha = 2 / (period + 1)` smoothing.
    pub fn new(period: usize) -> Self {
        assert!(period > 0, "EMA period must be > 0");
        Self { alpha: 2.0 / (period as f64 + 1.0), value: None }
    }
}

impl Incremental for Ema {
    type Input = f64;
    type Output = Option<f64>;

    fn update(&mut self, price: f64) -> Option<f64> {
        self.value = Some(match self.value {
            None => price,
            Some(prev) => self.alpha * price + (1.0 - self.alpha) * prev,
        });
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
    fn first_update_seeds_with_the_price() {
        let mut ema = Ema::new(10);
        assert_eq!(ema.update(100.0), Some(100.0));
    }

    #[test]
    fn converges_toward_a_constant_input() {
        let mut ema = Ema::new(5);
        let mut last = 0.0;
        for _ in 0..200 {
            last = ema.update(50.0).unwrap();
        }
        assert!((last - 50.0).abs() < 1e-6);
    }

    #[test]
    fn matches_hand_computed_second_step() {
        // alpha = 2/(3+1) = 0.5 for period=3
        let mut ema = Ema::new(3);
        ema.update(10.0);
        let v = ema.update(20.0).unwrap();
        assert!((v - 15.0).abs() < 1e-9);
    }
}
