use std::collections::VecDeque;

use crate::atr::OhlcInput;

/// Fractal swing high/low detection (§8.1 "Market structure": "Swing
/// high/low sequence... fractal pivots"). A bar `k` bars back is confirmed
/// a swing high once its high is the maximum of the `2k+1`-bar window
/// centered on it (swing low: symmetric on lows) — which means every
/// swing point is necessarily reported `k` bars late. That lag is
/// intrinsic to swing detection, not a shortcoming of this implementation:
/// you cannot know a bar was a local extreme until you've seen what came
/// after it.
pub struct SwingDetector {
    k: usize,
    window: VecDeque<OhlcInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SwingSignal {
    pub swing_high: Option<f64>,
    pub swing_low: Option<f64>,
}

impl SwingDetector {
    pub fn new(k: usize) -> Self {
        assert!(k > 0);
        Self { k, window: VecDeque::with_capacity(2 * k + 1) }
    }

    /// Feeds one bar in. Returns the swing signal for the bar `k` positions
    /// back, once enough surrounding context exists to confirm it.
    pub fn update(&mut self, bar: OhlcInput) -> SwingSignal {
        self.window.push_back(bar);
        if self.window.len() < 2 * self.k + 1 {
            return SwingSignal::default();
        }
        let mid = self.window[self.k];
        let is_swing_high = self.window.iter().all(|b| b.high <= mid.high);
        let is_swing_low = self.window.iter().all(|b| b.low >= mid.low);
        self.window.pop_front();
        SwingSignal {
            swing_high: is_swing_high.then_some(mid.high),
            swing_low: is_swing_low.then_some(mid.low),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(high: f64, low: f64) -> OhlcInput {
        OhlcInput { high, low, close: (high + low) / 2.0 }
    }

    #[test]
    fn no_signal_before_the_window_fills() {
        let mut d = SwingDetector::new(2);
        for h in [10.0, 12.0, 15.0] {
            assert_eq!(d.update(bar(h, h - 1.0)), SwingSignal::default());
        }
    }

    #[test]
    fn detects_a_clean_swing_high() {
        let mut d = SwingDetector::new(2);
        // highs: 10, 12, 15 (peak), 11, 9 -> window of 5 confirms bar index 2 (15) as a swing high.
        let mut last = SwingSignal::default();
        for h in [10.0, 12.0, 15.0, 11.0, 9.0] {
            last = d.update(bar(h, h - 5.0));
        }
        assert_eq!(last.swing_high, Some(15.0));
    }

    #[test]
    fn detects_a_clean_swing_low() {
        let mut d = SwingDetector::new(2);
        // lows: 20, 15, 8 (trough), 14, 19
        let mut last = SwingSignal::default();
        for l in [20.0, 15.0, 8.0, 14.0, 19.0] {
            last = d.update(bar(l + 5.0, l));
        }
        assert_eq!(last.swing_low, Some(8.0));
    }

    #[test]
    fn a_monotonic_run_has_no_interior_swing_point() {
        let mut d = SwingDetector::new(2);
        let mut last = SwingSignal::default();
        for h in [10.0, 11.0, 12.0, 13.0, 14.0] {
            last = d.update(bar(h, h - 1.0));
        }
        assert_eq!(last, SwingSignal::default());
    }
}
