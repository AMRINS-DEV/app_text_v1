//! TradingView Pine Script "alert webhook" ingestion (§5.4 point 2:
//! "ingesting TradingView webhook alerts as external signals"). TradingView
//! alerts are arbitrary POST bodies a user types into the Pine alert
//! dialog's message template — there is no fixed schema on TradingView's
//! side, so this project defines its own expected JSON shape (below) that a
//! user's alert message template must produce.
//!
//! Auth is a shared-secret token compared against the payload's `token`
//! field — a real minimum bar, not a full HMAC-signed webhook scheme. That
//! is an honest, narrower scope than a production integration would want,
//! the same "real logic, realistically scoped edges" discipline used
//! throughout this project (e.g. `agents_validation.backtest`'s fixed
//! cost-in-R constant standing in for a full market-impact model).

use domain::{SymbolId, Tick};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct WebhookAlert {
    pub token: String,
    pub ticker: String,
    pub price: f64,
    pub time_ns: u64,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum WebhookError {
    #[error("unauthorized: token mismatch")]
    Unauthorized,
}

pub fn authenticate<'a>(alert: &'a WebhookAlert, expected_token: &str) -> Result<&'a WebhookAlert, WebhookError> {
    if alert.token != expected_token {
        return Err(WebhookError::Unauthorized);
    }
    Ok(alert)
}

/// Converts a validated alert into a `Tick` for the symbol it names. A
/// TradingView alert fires *at* a specific price, so that price is a real,
/// if single-sided, market observation for that instant — `bid`/`ask` are
/// both set to it rather than fabricating an independent spread this source
/// never reported.
pub fn alert_to_tick(alert: &WebhookAlert, symbol_id: SymbolId, price_scale: i64) -> Tick {
    let px = (alert.price * price_scale as f64).round() as i64;
    Tick { ts_ns: alert.time_ns, recv_ns: alert.time_ns, symbol_id, bid: px, ask: px, bid_volume: 0, ask_volume: 0, flags: 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alert() -> WebhookAlert {
        WebhookAlert { token: "secret".into(), ticker: "EURUSD".into(), price: 1.23456, time_ns: 42_000_000_000 }
    }

    #[test]
    fn matching_token_authenticates() {
        assert_eq!(authenticate(&alert(), "secret"), Ok(&alert()));
    }

    #[test]
    fn mismatched_token_is_rejected() {
        assert_eq!(authenticate(&alert(), "wrong"), Err(WebhookError::Unauthorized));
    }

    #[test]
    fn alert_price_is_scaled_into_fixed_point_and_bid_equals_ask() {
        let tick = alert_to_tick(&alert(), 7, 100_000);
        assert_eq!(tick.symbol_id, 7);
        assert_eq!(tick.bid, 123_456);
        assert_eq!(tick.ask, tick.bid);
        assert_eq!(tick.ts_ns, 42_000_000_000);
    }
}
