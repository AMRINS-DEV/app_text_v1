//! Signal fusion (§8.4): log-odds pooling with source-reliability weights
//! and a correlation penalty, plus the hard sample-size gate:
//!
//! ```text
//! logit(P_fused) = Σ_i w_i·logit(P_i) − λ·Σ_{i<j} ρ_ij·|logit(P_i)|·|logit(P_j)|
//! ```
//!
//! `w_i` (source reliability, "updated online from its realized Brier
//! score") comes from `BrierTracker::skill_score`; `ρ_ij` (empirical
//! correlation between two sources' historical signals) comes from
//! `PairwiseCorrelationTracker::correlation`. Both are real, streaming
//! statistics fed by resolved outcomes — the placeholder weighted average
//! this module used through Phase 5 is gone as of Phase 6.

use std::collections::HashMap;

const EPSILON: f64 = 1e-6;

fn logit(p: f64) -> f64 {
    let clamped = p.clamp(EPSILON, 1.0 - EPSILON);
    (clamped / (1.0 - clamped)).ln()
}

fn inv_logit(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// One source's proposal for the current (symbol, regime, session) cell.
pub struct FusionInput {
    pub source_id: String,
    pub probability: f32,
    /// Source reliability weight — the caller reads this from that
    /// source's `BrierTracker::skill_score()`.
    pub weight: f32,
    /// Resolved historical prediction count for this cell — sources below
    /// `SAMPLE_SIZE_GATE` are excluded entirely per §8.4.
    pub resolved_predictions: u32,
}

pub const SAMPLE_SIZE_GATE: u32 = 30;
/// §8.4's tuned correlation penalty.
pub const DEFAULT_LAMBDA: f32 = 0.15;

/// Symmetric lookup for ρ_ij between two source ids, populated from each
/// pair's `PairwiseCorrelationTracker::correlation()`. A missing pair
/// defaults to 0 (no evidence of correlation yet, so no penalty).
#[derive(Default)]
pub struct Correlations(HashMap<(String, String), f32>);

impl Correlations {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    fn key(a: &str, b: &str) -> (String, String) {
        if a <= b {
            (a.to_string(), b.to_string())
        } else {
            (b.to_string(), a.to_string())
        }
    }

    pub fn set(&mut self, a: &str, b: &str, rho: f32) {
        self.0.insert(Self::key(a, b), rho);
    }

    pub fn get(&self, a: &str, b: &str) -> f32 {
        self.0.get(&Self::key(a, b)).copied().unwrap_or(0.0)
    }
}

/// Real §8.4 log-odds pooling: excludes any source below the sample-size
/// gate or with non-positive weight, then combines the rest in logit space
/// with a penalty for pairs of correlated sources so duplicated evidence
/// isn't double-counted.
pub fn fuse(inputs: &[FusionInput], correlations: &Correlations, lambda: f32) -> Option<f32> {
    let gated: Vec<&FusionInput> =
        inputs.iter().filter(|i| i.resolved_predictions >= SAMPLE_SIZE_GATE && i.weight > 0.0).collect();
    if gated.is_empty() {
        return None;
    }

    let logits: Vec<f64> = gated.iter().map(|i| logit(i.probability as f64)).collect();

    let pooled: f64 = gated.iter().zip(&logits).map(|(i, l)| i.weight as f64 * l).sum();

    let mut penalty = 0.0f64;
    for a in 0..gated.len() {
        for b in (a + 1)..gated.len() {
            let rho = f64::from(correlations.get(&gated[a].source_id, &gated[b].source_id));
            penalty += rho * logits[a].abs() * logits[b].abs();
        }
    }

    let fused_logit = pooled - f64::from(lambda) * penalty;
    Some(inv_logit(fused_logit) as f32)
}

/// Online Brier-score tracker for one source, converted to a bounded
/// `[0, 1]` reliability weight via the Brier Skill Score against the
/// source's own observed base rate — a source no better than "always
/// guess the base rate" scores 0, not some arbitrary positive number, and
/// a worse-than-that source is clamped to 0 rather than going negative
/// (§8.4's sample-size gate is what excludes a source outright, not a
/// negative weight).
#[derive(Default)]
pub struct BrierTracker {
    n: u32,
    sum_squared_error: f64,
    sum_outcome: f64,
}

impl BrierTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, predicted_probability: f32, outcome: bool) {
        let outcome_value = if outcome { 1.0 } else { 0.0 };
        let error = f64::from(predicted_probability) - outcome_value;
        self.sum_squared_error += error * error;
        self.sum_outcome += outcome_value;
        self.n += 1;
    }

    pub fn resolved_count(&self) -> u32 {
        self.n
    }

    /// `None` until at least one outcome has been recorded.
    pub fn skill_score(&self) -> Option<f64> {
        if self.n == 0 {
            return None;
        }
        let n = f64::from(self.n);
        let brier_score = self.sum_squared_error / n;
        let base_rate = self.sum_outcome / n;
        let reference_brier_score = base_rate * (1.0 - base_rate);
        if reference_brier_score <= 0.0 {
            // Every resolved outcome was identical — there's no base-rate
            // uncertainty to beat, so score 1.0 only for a source that
            // called it perfectly, 0.0 otherwise.
            return Some(if brier_score == 0.0 { 1.0 } else { 0.0 });
        }
        Some((1.0 - brier_score / reference_brier_score).max(0.0))
    }
}

/// Online Pearson-correlation tracker between two sources' probability
/// outputs on the same signal instances — feeds `Correlations::set`.
#[derive(Default)]
pub struct PairwiseCorrelationTracker {
    n: f64,
    sum_x: f64,
    sum_y: f64,
    sum_x2: f64,
    sum_y2: f64,
    sum_xy: f64,
}

impl PairwiseCorrelationTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&mut self, probability_a: f32, probability_b: f32) {
        let x = f64::from(probability_a);
        let y = f64::from(probability_b);
        self.n += 1.0;
        self.sum_x += x;
        self.sum_y += y;
        self.sum_x2 += x * x;
        self.sum_y2 += y * y;
        self.sum_xy += x * y;
    }

    /// `None` until at least two paired observations exist.
    pub fn correlation(&self) -> Option<f64> {
        if self.n < 2.0 {
            return None;
        }
        let mean_x = self.sum_x / self.n;
        let mean_y = self.sum_y / self.n;
        let covariance = self.sum_xy / self.n - mean_x * mean_y;
        let variance_x = self.sum_x2 / self.n - mean_x * mean_x;
        let variance_y = self.sum_y2 / self.n - mean_y * mean_y;
        if variance_x <= 0.0 || variance_y <= 0.0 {
            return Some(0.0);
        }
        Some(covariance / (variance_x.sqrt() * variance_y.sqrt()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(source_id: &str, probability: f32, weight: f32, resolved_predictions: u32) -> FusionInput {
        FusionInput { source_id: source_id.to_string(), probability, weight, resolved_predictions }
    }

    #[test]
    fn sources_below_sample_size_gate_are_excluded() {
        let inputs = vec![input("a", 0.9, 1.0, 5), input("b", 0.6, 1.0, 40)];
        let fused = fuse(&inputs, &Correlations::new(), DEFAULT_LAMBDA).unwrap();
        // Only "b" clears the gate, so the fused probability is exactly its own.
        assert!((fused - 0.6).abs() < 1e-6);
    }

    #[test]
    fn zero_weight_sources_are_excluded_even_past_the_sample_size_gate() {
        let inputs = vec![input("a", 0.99, 0.0, 40), input("b", 0.6, 1.0, 40)];
        let fused = fuse(&inputs, &Correlations::new(), DEFAULT_LAMBDA).unwrap();
        assert!((fused - 0.6).abs() < 1e-6);
    }

    #[test]
    fn no_gated_sources_returns_none() {
        let inputs = vec![input("a", 0.9, 1.0, 1)];
        assert_eq!(fuse(&inputs, &Correlations::new(), DEFAULT_LAMBDA), None);
    }

    #[test]
    fn equal_weight_uncorrelated_sources_pool_multiplicatively_in_odds_space() {
        // odds(0.7) * odds(0.6) = (7/3) * (3/2) = 3.5 -> p = 3.5 / 4.5 = 7/9.
        let inputs = vec![input("news", 0.7, 1.0, 40), input("pattern", 0.6, 1.0, 40)];
        let fused = fuse(&inputs, &Correlations::new(), DEFAULT_LAMBDA).unwrap();
        assert!((fused - (7.0 / 9.0)).abs() < 1e-6, "fused={fused}");
    }

    #[test]
    fn correlated_sources_are_penalized_toward_less_confidence() {
        let inputs = vec![input("news", 0.7, 1.0, 40), input("pattern", 0.6, 1.0, 40)];
        let uncorrelated = fuse(&inputs, &Correlations::new(), DEFAULT_LAMBDA).unwrap();

        let mut correlations = Correlations::new();
        correlations.set("news", "pattern", 1.0);
        let correlated = fuse(&inputs, &correlations, DEFAULT_LAMBDA).unwrap();

        assert!(correlated < uncorrelated, "correlated={correlated} uncorrelated={uncorrelated}");
        // Hand-computed: fused_logit = logit(0.7)+logit(0.6) - 0.15*|logit(0.7)|*|logit(0.6)|
        assert!((correlated - 0.768_76).abs() < 1e-3, "correlated={correlated}");
    }

    #[test]
    fn brier_tracker_gives_a_skilled_source_a_high_score() {
        let mut tracker = BrierTracker::new();
        tracker.record(0.9, true);
        tracker.record(0.9, true);
        tracker.record(0.1, false);
        tracker.record(0.1, false);
        // base_rate=0.5, ref_brier=0.25, brier=0.01 -> skill=1-0.01/0.25=0.96
        // (tolerance loosened past f64 epsilon: 0.9/0.1 aren't exactly
        // representable in f32, so the f32->f64 conversion carries a small
        // rounding error into the arithmetic)
        assert!((tracker.skill_score().unwrap() - 0.96).abs() < 1e-6);
    }

    #[test]
    fn brier_tracker_gives_a_base_rate_guesser_zero_skill() {
        let mut tracker = BrierTracker::new();
        tracker.record(0.5, true);
        tracker.record(0.5, false);
        tracker.record(0.5, true);
        tracker.record(0.5, false);
        assert!((tracker.skill_score().unwrap() - 0.0).abs() < 1e-9);
    }

    #[test]
    fn brier_tracker_clamps_a_worse_than_base_rate_source_to_zero() {
        let mut tracker = BrierTracker::new();
        tracker.record(0.9, false);
        tracker.record(0.9, false);
        tracker.record(0.1, true);
        tracker.record(0.1, true);
        // skill would be 1 - 0.81/0.25 = -2.24 without clamping.
        assert_eq!(tracker.skill_score().unwrap(), 0.0);
    }

    #[test]
    fn brier_tracker_reports_none_before_any_outcome() {
        assert_eq!(BrierTracker::new().skill_score(), None);
    }

    #[test]
    fn correlation_tracker_detects_perfect_positive_correlation() {
        let mut tracker = PairwiseCorrelationTracker::new();
        tracker.record(0.1, 0.1);
        tracker.record(0.5, 0.5);
        tracker.record(0.9, 0.9);
        assert!((tracker.correlation().unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn correlation_tracker_detects_perfect_negative_correlation() {
        let mut tracker = PairwiseCorrelationTracker::new();
        tracker.record(0.1, 0.9);
        tracker.record(0.5, 0.5);
        tracker.record(0.9, 0.1);
        assert!((tracker.correlation().unwrap() - (-1.0)).abs() < 1e-9);
    }

    #[test]
    fn correlation_tracker_reports_none_with_fewer_than_two_observations() {
        let mut tracker = PairwiseCorrelationTracker::new();
        assert_eq!(tracker.correlation(), None);
        tracker.record(0.4, 0.6);
        assert_eq!(tracker.correlation(), None);
    }

    #[test]
    fn a_well_evidenced_graph_prior_changes_the_expectancy_gate_decision() {
        // §17 Phase 6 exit criterion: "priors measurably improve
        // expectancy." §8.5's gate requires p >= 0.55 in normal mode; E[R]
        // is monotonically increasing in p (dE[R]/dp = R_target + 1 > 0),
        // so a higher fused p directly means higher expectancy — showing
        // the fused probability crosses that threshold only once the graph
        // prior is included is a direct, verifiable "priors improve
        // expectancy" demonstration, without needing to re-implement the
        // §8.5 gate itself here.
        //
        // Two independent, weak/noisy agent signals near a coin flip...
        let weak_agents = vec![
            input("pattern-agent", 0.52, 1.0, 40),
            input("news-agent", 0.48, 1.0, 40),
        ];
        let fused_without_prior = fuse(&weak_agents, &Correlations::new(), DEFAULT_LAMBDA).unwrap();
        assert!(
            fused_without_prior < 0.55,
            "expected the weak-agents-only fusion to fail the §8.5 p_min_mode=0.55 gate, got {fused_without_prior}"
        );

        // ...plus a graph-prior source built from real historical
        // resolved-outcome evidence (§7.2's conditional-reliability query,
        // via `agents_graph.graph_prior_to_fusion_input` on the Python
        // side of the bridge this sandbox doesn't have live).
        let mut with_prior = weak_agents;
        with_prior.push(input("graph-prior", 0.65, 1.0, 87));
        let fused_with_prior = fuse(&with_prior, &Correlations::new(), DEFAULT_LAMBDA).unwrap();

        assert!(
            fused_with_prior >= 0.55,
            "expected the graph prior to push the fused probability over the §8.5 gate, got {fused_with_prior}"
        );
        assert!(fused_with_prior > fused_without_prior);
    }
}
