use crate::ring::MinMaxRing;
use crate::Incremental;

/// Donchian Channel: highest high / lowest low over `period` bars (§8.1
/// "Donchian position").
pub struct Donchian {
    highs: MinMaxRing,
    lows: MinMaxRing,
    period: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DonchianChannel {
    pub upper: f64,
    pub lower: f64,
}

impl DonchianChannel {
    /// Where `close` sits inside the channel, in [0,1] (0 = at the lower
    /// band, 1 = at the upper band) — the "Donchian position" feature.
    pub fn position(&self, close: f64) -> f64 {
        let range = self.upper - self.lower;
        if range <= 0.0 {
            return 0.5;
        }
        ((close - self.lower) / range).clamp(0.0, 1.0)
    }
}

impl Donchian {
    pub fn new(period: usize) -> Self {
        Self { highs: MinMaxRing::new(period), lows: MinMaxRing::new(period), period }
    }
}

impl Incremental for Donchian {
    type Input = (f64, f64); // (high, low)
    type Output = Option<DonchianChannel>;

    fn update(&mut self, (high, low): Self::Input) -> Self::Output {
        self.highs.push(high);
        self.lows.push(low);
        self.value()
    }

    fn value(&self) -> Self::Output {
        if self.highs.len() < self.period {
            return None;
        }
        Some(DonchianChannel { upper: self.highs.max()?, lower: self.lows.min()? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warms_up_before_emitting() {
        let mut d = Donchian::new(3);
        assert_eq!(d.update((10.0, 9.0)), None);
        assert_eq!(d.update((11.0, 8.0)), None);
        assert!(d.update((12.0, 7.0)).is_some());
    }

    #[test]
    fn tracks_the_channel_over_the_window() {
        let mut d = Donchian::new(3);
        for (h, l) in [(10.0, 9.0), (15.0, 8.0), (12.0, 6.0), (11.0, 7.0)] {
            d.update((h, l));
        }
        // window is the last 3: highs [15,12,11] lows [8,6,7]
        let ch = d.value().unwrap();
        assert_eq!(ch.upper, 15.0);
        assert_eq!(ch.lower, 6.0);
    }

    #[test]
    fn position_is_zero_at_lower_one_at_upper() {
        let ch = DonchianChannel { upper: 110.0, lower: 100.0 };
        assert!((ch.position(100.0) - 0.0).abs() < 1e-9);
        assert!((ch.position(110.0) - 1.0).abs() < 1e-9);
        assert!((ch.position(105.0) - 0.5).abs() < 1e-9);
    }
}
