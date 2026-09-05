use crate::Incremental;

/// Volume-Weighted Average Price, reset per session (§8.1 "Session &
/// time": VWAP is meaningless carried across a session boundary). O(1)
/// exactly, not just amortized — it's two running sums.
#[derive(Debug, Clone, Copy, Default)]
pub struct Vwap {
    cumulative_pv: f64,
    cumulative_volume: f64,
}

impl Vwap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts a new session (§8.1: session one-hot / minutes-since-open —
    /// this is the corresponding reset for VWAP itself).
    pub fn reset(&mut self) {
        self.cumulative_pv = 0.0;
        self.cumulative_volume = 0.0;
    }
}

impl Incremental for Vwap {
    type Input = (f64, f64); // (price, volume)
    type Output = Option<f64>;

    fn update(&mut self, (price, volume): Self::Input) -> Self::Output {
        self.cumulative_pv += price * volume;
        self.cumulative_volume += volume;
        self.value()
    }

    fn value(&self) -> Self::Output {
        if self.cumulative_volume <= 0.0 {
            return None;
        }
        Some(self.cumulative_pv / self.cumulative_volume)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_volume_yet_is_none() {
        let vwap = Vwap::new();
        assert_eq!(vwap.value(), None);
    }

    #[test]
    fn matches_hand_computed_weighted_average() {
        let mut vwap = Vwap::new();
        vwap.update((10.0, 100.0));
        vwap.update((20.0, 300.0));
        // (10*100 + 20*300) / (100+300) = (1000+6000)/400 = 17.5
        assert!((vwap.value().unwrap() - 17.5).abs() < 1e-9);
    }

    #[test]
    fn reset_clears_accumulated_state_for_a_new_session() {
        let mut vwap = Vwap::new();
        vwap.update((10.0, 100.0));
        vwap.reset();
        assert_eq!(vwap.value(), None);
        vwap.update((50.0, 10.0));
        assert!((vwap.value().unwrap() - 50.0).abs() < 1e-9);
    }
}
