//! Small fixed-capacity ring buffers shared by the window-based indicators
//! (Bollinger, Donchian, Efficiency Ratio). "O(1) incremental" for these
//! means *amortized* O(1) per update via a maintained running
//! sum/sum-of-squares or a monotonic deque — never recomputing the whole
//! window from scratch, which is the actual rule §5.1/§8.1 care about.

use std::collections::VecDeque;

/// Fixed-capacity window that tracks a running sum and sum-of-squares, so
/// mean/variance are O(1) reads regardless of window size.
#[derive(Debug, Clone)]
pub(crate) struct SumRing {
    capacity: usize,
    values: VecDeque<f64>,
    sum: f64,
    sum_sq: f64,
}

impl SumRing {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self { capacity, values: VecDeque::with_capacity(capacity), sum: 0.0, sum_sq: 0.0 }
    }

    pub fn push(&mut self, value: f64) {
        self.values.push_back(value);
        self.sum += value;
        self.sum_sq += value * value;
        if self.values.len() > self.capacity {
            if let Some(evicted) = self.values.pop_front() {
                self.sum -= evicted;
                self.sum_sq -= evicted * evicted;
            }
        }
    }

    pub fn is_full(&self) -> bool {
        self.values.len() == self.capacity
    }

    pub fn mean(&self) -> f64 {
        self.sum / self.values.len() as f64
    }

    /// Population variance (matches Bollinger's conventional stddev, which
    /// uses N not N-1).
    pub fn variance(&self) -> f64 {
        let n = self.values.len() as f64;
        let mean = self.mean();
        (self.sum_sq / n - mean * mean).max(0.0) // clamp: FP error can make this tiny-negative
    }
}

/// Fixed-capacity window that tracks running max and min in amortized O(1)
/// per push, via two monotonic deques of (value) — the classic sliding-
/// window-maximum technique, doubled up for the minimum too (Donchian
/// needs both simultaneously).
#[derive(Debug, Clone)]
pub(crate) struct MinMaxRing {
    capacity: usize,
    index: usize,
    max_deque: VecDeque<(usize, f64)>,
    min_deque: VecDeque<(usize, f64)>,
}

impl MinMaxRing {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0);
        Self { capacity, index: 0, max_deque: VecDeque::new(), min_deque: VecDeque::new() }
    }

    pub fn push(&mut self, value: f64) {
        while self.max_deque.back().is_some_and(|&(_, v)| v <= value) {
            self.max_deque.pop_back();
        }
        self.max_deque.push_back((self.index, value));
        while self.min_deque.back().is_some_and(|&(_, v)| v >= value) {
            self.min_deque.pop_back();
        }
        self.min_deque.push_back((self.index, value));

        let window_start = self.index.saturating_sub(self.capacity - 1);
        while self.max_deque.front().is_some_and(|&(i, _)| i < window_start) {
            self.max_deque.pop_front();
        }
        while self.min_deque.front().is_some_and(|&(i, _)| i < window_start) {
            self.min_deque.pop_front();
        }
        self.index += 1;
    }

    pub fn max(&self) -> Option<f64> {
        self.max_deque.front().map(|&(_, v)| v)
    }

    pub fn min(&self) -> Option<f64> {
        self.min_deque.front().map(|&(_, v)| v)
    }

    pub fn len(&self) -> usize {
        self.index.min(self.capacity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sum_ring_mean_and_variance() {
        let mut r = SumRing::new(3);
        for v in [2.0, 4.0, 6.0] {
            r.push(v);
        }
        assert!((r.mean() - 4.0).abs() < 1e-9);
        // population variance of [2,4,6]: mean=4, sq diffs 4,0,4 -> mean 8/3
        assert!((r.variance() - 8.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn sum_ring_evicts_oldest_beyond_capacity() {
        let mut r = SumRing::new(2);
        r.push(10.0);
        r.push(20.0);
        r.push(30.0); // evicts 10.0
        assert!((r.mean() - 25.0).abs() < 1e-9);
    }

    #[test]
    fn min_max_ring_tracks_a_sliding_window() {
        let mut r = MinMaxRing::new(3);
        for v in [5.0, 1.0, 4.0, 2.0, 8.0] {
            r.push(v);
        }
        // window is the last 3 pushed: [4, 2, 8]
        assert_eq!(r.max(), Some(8.0));
        assert_eq!(r.min(), Some(2.0));
    }
}
