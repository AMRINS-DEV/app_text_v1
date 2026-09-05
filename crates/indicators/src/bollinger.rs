use crate::ring::SumRing;
use crate::Incremental;

/// Bollinger Bands: SMA ± `k` standard deviations over `period`. §8.1
/// "Bollinger bandwidth percentile" is computed downstream (in
/// `crates/features`) from `width()`, not here — this type only tracks the
/// bands themselves.
pub struct Bollinger {
    window: SumRing,
    k: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BollingerBands {
    pub middle: f64,
    pub upper: f64,
    pub lower: f64,
}

impl BollingerBands {
    pub fn width(&self) -> f64 {
        self.upper - self.lower
    }
}

impl Bollinger {
    pub fn new(period: usize, k: f64) -> Self {
        Self { window: SumRing::new(period), k }
    }
}

impl Incremental for Bollinger {
    type Input = f64;
    type Output = Option<BollingerBands>;

    fn update(&mut self, price: f64) -> Self::Output {
        self.window.push(price);
        self.value()
    }

    fn value(&self) -> Self::Output {
        if !self.window.is_full() {
            return None;
        }
        let middle = self.window.mean();
        let stddev = self.window.variance().sqrt();
        Some(BollingerBands { middle, upper: middle + self.k * stddev, lower: middle - self.k * stddev })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warms_up_before_emitting() {
        let mut b = Bollinger::new(3, 2.0);
        assert_eq!(b.update(10.0), None);
        assert_eq!(b.update(10.0), None);
        assert!(b.update(10.0).is_some());
    }

    #[test]
    fn constant_price_gives_zero_width_bands() {
        let mut b = Bollinger::new(3, 2.0);
        for _ in 0..3 {
            b.update(50.0);
        }
        let bands = b.value().unwrap();
        assert!((bands.middle - 50.0).abs() < 1e-9);
        assert!((bands.width()).abs() < 1e-9);
    }

    #[test]
    fn upper_band_is_above_middle_is_above_lower() {
        let mut b = Bollinger::new(3, 2.0);
        for p in [10.0, 20.0, 15.0] {
            b.update(p);
        }
        let bands = b.value().unwrap();
        assert!(bands.upper > bands.middle);
        assert!(bands.middle > bands.lower);
    }
}
