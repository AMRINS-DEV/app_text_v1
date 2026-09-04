//! Declarative strategy configuration (§5.5). Strategies are data, not
//! compiled Rust, so they can be added/tuned from the dashboard without
//! redeploying the core.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct StrategyConfig {
    pub id: String,
    pub symbols: Vec<String>,
    pub modes: Vec<String>,
    pub sessions: Vec<String>,
    pub entry: EntryConfig,
    #[serde(default)]
    pub veto_any: Vec<VetoRule>,
    pub exit: ExitConfig,
    pub sizing: SizingConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct EntryConfig {
    pub require_all: Vec<serde_yaml::Value>,
}

pub type VetoRule = serde_yaml::Value;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ExitConfig {
    pub stop: serde_yaml::Value,
    pub target: serde_yaml::Value,
    #[serde(default)]
    pub trailing: Option<serde_yaml::Value>,
    #[serde(default)]
    pub breakeven: Option<serde_yaml::Value>,
    /// §9.4: gated, partial by default — never a full close unless
    /// `regime_gate` restricts it to ranging/choppy regimes.
    #[serde(default)]
    pub quick_profit: Option<serde_yaml::Value>,
    #[serde(default)]
    pub time_stop: Option<serde_yaml::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct SizingConfig {
    pub method: String,
    pub kelly_fraction: f64,
    pub risk_per_trade_pct: f64,
    pub max_concurrent: u32,
}

impl StrategyConfig {
    pub fn from_yaml(s: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LONDON_BREAKOUT: &str = r#"
id: london_breakout_v3
symbols: [XAUUSD, EURUSD, GBPUSD]
modes: [normal, aggressive]
sessions: [london_open]
entry:
  require_all:
    - regime: [trending, expansion]
    - feature: atr_percentile
      op: between
      value: [0.35, 0.90]
veto_any:
  - news_blackout: {minutes_before: 15, minutes_after: 10, impact: [high]}
exit:
  stop: {type: atr, mult: 1.5, min_pts: 30}
  target: {type: r_multiple, value: 2.2}
  quick_profit: {enabled: true, trigger_r: 0.6, close_fraction: 0.5}
sizing:
  method: fractional_kelly
  kelly_fraction: 0.25
  risk_per_trade_pct: 0.5
  max_concurrent: 3
"#;

    #[test]
    fn parses_the_design_docs_own_example_config() {
        let cfg = StrategyConfig::from_yaml(LONDON_BREAKOUT).unwrap();
        assert_eq!(cfg.id, "london_breakout_v3");
        assert_eq!(cfg.symbols, vec!["XAUUSD", "EURUSD", "GBPUSD"]);
        assert_eq!(cfg.sizing.kelly_fraction, 0.25);
        assert_eq!(cfg.veto_any.len(), 1);
    }
}
