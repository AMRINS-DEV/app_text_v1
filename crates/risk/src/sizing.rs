//! Fractional-Kelly position sizing (§9.2). Full Kelly is mathematically
//! optimal for growth and practically ruinous — drawdowns of 50%+. Quarter
//! Kelly (`kappa = 0.25`) captures ~90% of the growth at ~25% of the
//! drawdown. **Never expose full Kelly (`kappa = 1.0`) in the UI.**

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SizingError {
    #[error("payoff ratio (R multiple) must be > 0, got {0}")]
    NonPositiveB(f64),
    #[error("probability must be in [0,1], got {0}")]
    ProbabilityOutOfRange(f64),
    #[error("stop distance must be > 0, got {0}")]
    NonPositiveStopDistance(f64),
}

#[derive(Debug, Clone, Copy)]
pub struct KellyInputs {
    /// Calibrated probability of the target being hit before the stop.
    pub probability: f64,
    /// Payoff ratio (R multiple), i.e. `b` in the Kelly formula.
    pub r_target: f64,
    /// Fraction of full Kelly to actually use. §9.2 mandates 0.25 (quarter Kelly).
    pub kappa: f64,
    /// Hard cap on the fraction of equity risked, regardless of what Kelly says.
    pub f_max: f64,
    pub equity: f64,
    /// Risk per trade as configured by the trading mode (§9.1), e.g. 0.005 for 0.5%.
    pub risk_per_trade_pct: f64,
    pub stop_distance: f64,
    pub pip_value: f64,
    pub contract_size: f64,
}

/// `f* = (p*(b+1) - 1) / b`, clamped to `[0, f_max]` after scaling by `kappa`.
pub fn kelly_fraction(probability: f64, r_target: f64, kappa: f64, f_max: f64) -> Result<f64, SizingError> {
    if !(0.0..=1.0).contains(&probability) {
        return Err(SizingError::ProbabilityOutOfRange(probability));
    }
    if r_target <= 0.0 {
        return Err(SizingError::NonPositiveB(r_target));
    }
    let b = r_target;
    let full_kelly = (probability * (b + 1.0) - 1.0) / b;
    Ok((kappa * full_kelly).clamp(0.0, f_max))
}

/// Converts a Kelly-derived risk fraction into a lot size, respecting the
/// mode's `risk_per_trade_pct` as an additional cap (§9.2's `min(f_use, risk_per_trade_pct)`).
/// Caller is responsible for the broker lot-step rounding (round DOWN, never up)
/// and the margin/correlation caps applied downstream in `crates/execution`.
pub fn kelly_lots(inputs: KellyInputs) -> Result<f64, SizingError> {
    if inputs.stop_distance <= 0.0 {
        return Err(SizingError::NonPositiveStopDistance(inputs.stop_distance));
    }
    let f_use = kelly_fraction(inputs.probability, inputs.r_target, inputs.kappa, inputs.f_max)?
        .min(inputs.risk_per_trade_pct);
    let risk_dollars = inputs.equity * f_use;
    Ok(risk_dollars / (inputs.stop_distance * inputs.pip_value * inputs.contract_size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_kelly_is_a_quarter_of_full_kelly() {
        let full = kelly_fraction(0.55, 2.2, 1.0, 1.0).unwrap();
        let quarter = kelly_fraction(0.55, 2.2, 0.25, 1.0).unwrap();
        assert!((quarter - full * 0.25).abs() < 1e-12);
    }

    #[test]
    fn negative_edge_clamps_to_zero_not_negative() {
        // p=0.3, b=1.0 => f* = (0.3*2 - 1)/1 = -0.4, must clamp to 0.
        let f = kelly_fraction(0.3, 1.0, 0.25, 1.0).unwrap();
        assert_eq!(f, 0.0);
    }

    #[test]
    fn f_max_is_a_hard_ceiling() {
        let f = kelly_fraction(0.9, 5.0, 1.0, 0.05).unwrap();
        assert!(f <= 0.05);
    }

    #[test]
    fn rejects_invalid_probability() {
        assert_eq!(kelly_fraction(1.5, 2.0, 0.25, 1.0), Err(SizingError::ProbabilityOutOfRange(1.5)));
    }

    #[test]
    fn rejects_non_positive_stop_distance() {
        let inputs = KellyInputs {
            probability: 0.6, r_target: 2.0, kappa: 0.25, f_max: 0.02,
            equity: 10_000.0, risk_per_trade_pct: 0.005,
            stop_distance: 0.0, pip_value: 10.0, contract_size: 100_000.0,
        };
        assert_eq!(kelly_lots(inputs), Err(SizingError::NonPositiveStopDistance(0.0)));
    }

    #[test]
    fn risk_per_trade_pct_caps_below_kelly() {
        // Kelly wants a large fraction, but risk_per_trade_pct (0.5%) must win.
        let inputs = KellyInputs {
            probability: 0.9, r_target: 5.0, kappa: 1.0, f_max: 1.0,
            equity: 10_000.0, risk_per_trade_pct: 0.005,
            stop_distance: 20.0, pip_value: 1.0, contract_size: 1.0,
        };
        let lots = kelly_lots(inputs).unwrap();
        // risk_dollars should be exactly equity * 0.005 = 50, not more.
        assert!((lots - 50.0 / 20.0).abs() < 1e-9);
    }
}
