//! Signal fusion (§8.4): log-odds pooling with source-reliability weights
//! and a correlation penalty, plus the hard sample-size gate. Stub pending
//! the online Brier-score weight tracker and empirical correlation matrix
//! from Phase 5/6 — the formula shape is fixed now so callers can be
//! written against it.

pub struct FusionInput {
    pub probability: f32,
    /// Source reliability weight, updated online from realized Brier score.
    pub weight: f32,
    /// Resolved historical prediction count for this (symbol, regime, session)
    /// cell — sources below 30 get `weight = 0` per §8.4's sample-size gate.
    pub resolved_predictions: u32,
}

pub const SAMPLE_SIZE_GATE: u32 = 30;

/// Placeholder: returns a naive weighted average once each input has cleared
/// the sample-size gate. Real log-odds pooling with the `rho_ij` correlation
/// penalty is Phase 5/6 scope (needs the online Brier tracker).
pub fn fuse(inputs: &[FusionInput]) -> Option<f32> {
    let gated: Vec<&FusionInput> = inputs.iter().filter(|i| i.resolved_predictions >= SAMPLE_SIZE_GATE).collect();
    if gated.is_empty() {
        return None;
    }
    let total_weight: f32 = gated.iter().map(|i| i.weight).sum();
    if total_weight <= 0.0 {
        return None;
    }
    Some(gated.iter().map(|i| i.probability * i.weight).sum::<f32>() / total_weight)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sources_below_sample_size_gate_are_excluded() {
        let inputs = vec![
            FusionInput { probability: 0.9, weight: 1.0, resolved_predictions: 5 },
            FusionInput { probability: 0.6, weight: 1.0, resolved_predictions: 40 },
        ];
        assert_eq!(fuse(&inputs), Some(0.6));
    }

    #[test]
    fn no_gated_sources_returns_none() {
        let inputs = vec![FusionInput { probability: 0.9, weight: 1.0, resolved_predictions: 1 }];
        assert_eq!(fuse(&inputs), None);
    }
}
