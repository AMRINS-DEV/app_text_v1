//! TradingView UDF ("Universal Data Feed") datafeed server (§5.4 point 1
//! restated for the export direction: an actual TradingView chart, or any
//! tool speaking UDF, can point at this server to chart this project's own
//! bar data — the literal shape "UDF datafeed" names in §17's Phase 8
//! roadmap row). Unlike `adapter_ctrader::protocol`'s documented Protobuf
//! substitution, UDF is TradingView's real, publicly documented, small
//! JSON-over-HTTP protocol — simple enough to implement faithfully rather
//! than needing a stand-in.

use domain::Bar;
use serde::Serialize;

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct UdfConfig {
    pub supported_resolutions: Vec<String>,
    pub supports_search: bool,
    pub supports_group_request: bool,
    pub supports_marks: bool,
    pub supports_timescale_marks: bool,
    pub supports_time: bool,
}

impl Default for UdfConfig {
    fn default() -> Self {
        Self {
            supported_resolutions: vec!["1".into(), "5".into(), "15".into(), "60".into(), "1D".into()],
            supports_search: true,
            supports_group_request: false,
            supports_marks: false,
            supports_timescale_marks: false,
            supports_time: true,
        }
    }
}

#[derive(Debug, Serialize, Clone, PartialEq)]
pub struct UdfSymbolInfo {
    pub name: String,
    pub ticker: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub session: String,
    pub timezone: String,
    pub exchange: String,
    pub minmov: i64,
    pub pricescale: i64,
    pub has_intraday: bool,
    pub supported_resolutions: Vec<String>,
}

/// UDF's real `/history` response shape: `{"s":"ok", "t":[...], ...}` on
/// success or `{"s":"no_data"}` when the range is empty — an internally
/// tagged enum on the `s` field reproduces this exactly.
#[derive(Debug, Serialize, Clone, PartialEq)]
#[serde(tag = "s")]
pub enum UdfHistoryResponse {
    #[serde(rename = "ok")]
    Ok { t: Vec<i64>, o: Vec<f64>, h: Vec<f64>, l: Vec<f64>, c: Vec<f64>, v: Vec<f64> },
    #[serde(rename = "no_data")]
    NoData,
}

/// UDF resolution strings ("1", "5", "60", "1D", ...) onto this project's
/// own `timeframe_seconds` bar key — the one piece of protocol translation
/// needed to serve `domain::Bar`s through UDF's format.
pub fn resolution_to_timeframe_seconds(resolution: &str) -> Option<u32> {
    match resolution {
        "1" => Some(60),
        "5" => Some(300),
        "15" => Some(900),
        "60" => Some(3_600),
        "1D" | "D" => Some(86_400),
        _ => None,
    }
}

/// Groups already-closed bars (assumed ascending by `ts_open_ns`, at
/// whatever their native timeframe is) into coarser `target_timeframe_seconds`
/// buckets — real resampling, not just relabeling, so a UDF client that
/// requests "60" or "1D" gets genuinely aggregated OHLCV rather than the
/// native-resolution bars under a different label.
pub fn resample(bars: &[Bar], target_timeframe_seconds: u32) -> Vec<Bar> {
    if bars.is_empty() {
        return Vec::new();
    }
    let tf_ns = target_timeframe_seconds as u64 * 1_000_000_000;
    let mut out: Vec<Bar> = Vec::new();
    for bar in bars {
        let bucket = (bar.ts_open_ns / tf_ns) * tf_ns;
        match out.last_mut() {
            Some(last) if last.ts_open_ns == bucket => {
                last.high = last.high.max(bar.high);
                last.low = last.low.min(bar.low);
                last.close = bar.close;
                last.volume += bar.volume;
            }
            _ => out.push(Bar {
                symbol_id: bar.symbol_id,
                timeframe_seconds: target_timeframe_seconds,
                ts_open_ns: bucket,
                open: bar.open,
                high: bar.high,
                low: bar.low,
                close: bar.close,
                volume: bar.volume,
            }),
        }
    }
    out
}

pub fn bars_to_history_response(bars: &[Bar], from_ns: u64, to_ns: u64, price_scale: i64) -> UdfHistoryResponse {
    let filtered: Vec<&Bar> = bars.iter().filter(|b| b.ts_open_ns >= from_ns && b.ts_open_ns <= to_ns).collect();
    if filtered.is_empty() {
        return UdfHistoryResponse::NoData;
    }
    let scale = price_scale as f64;
    UdfHistoryResponse::Ok {
        t: filtered.iter().map(|b| (b.ts_open_ns / 1_000_000_000) as i64).collect(),
        o: filtered.iter().map(|b| b.open as f64 / scale).collect(),
        h: filtered.iter().map(|b| b.high as f64 / scale).collect(),
        l: filtered.iter().map(|b| b.low as f64 / scale).collect(),
        c: filtered.iter().map(|b| b.close as f64 / scale).collect(),
        v: filtered.iter().map(|b| b.volume as f64).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(ts_open_ns: u64, open: i64, high: i64, low: i64, close: i64, volume: u64) -> Bar {
        Bar { symbol_id: 1, timeframe_seconds: 60, ts_open_ns, open, high, low, close, volume }
    }

    #[test]
    fn known_resolutions_map_to_the_right_second_count() {
        assert_eq!(resolution_to_timeframe_seconds("1"), Some(60));
        assert_eq!(resolution_to_timeframe_seconds("60"), Some(3_600));
        assert_eq!(resolution_to_timeframe_seconds("1D"), Some(86_400));
        assert_eq!(resolution_to_timeframe_seconds("bogus"), None);
    }

    #[test]
    fn resampling_to_the_same_timeframe_is_a_no_op() {
        let bars = vec![bar(0, 100, 110, 90, 105, 10), bar(60_000_000_000, 105, 115, 100, 110, 20)];
        let resampled = resample(&bars, 60);
        assert_eq!(resampled, bars);
    }

    #[test]
    fn resampling_five_one_minute_bars_into_a_five_minute_bar_aggregates_correctly() {
        let one_min: Vec<Bar> = (0i64..5)
            .map(|i| bar(i as u64 * 60_000_000_000, 100 + i, 100 + i + 5, 100 + i - 5, 100 + i + 1, 10))
            .collect();
        let resampled = resample(&one_min, 300);
        assert_eq!(resampled.len(), 1);
        let bucket = &resampled[0];
        assert_eq!(bucket.open, one_min[0].open); // first bar's open
        assert_eq!(bucket.close, one_min[4].close); // last bar's close
        assert_eq!(bucket.high, one_min.iter().map(|b| b.high).max().unwrap());
        assert_eq!(bucket.low, one_min.iter().map(|b| b.low).min().unwrap());
        assert_eq!(bucket.volume, 50); // sum of all five bars' volume
    }

    #[test]
    fn empty_history_range_reports_no_data_not_an_empty_ok() {
        let bars = vec![bar(0, 100, 100, 100, 100, 1)];
        let response = bars_to_history_response(&bars, 1_000_000_000_000, 2_000_000_000_000, 100_000);
        assert_eq!(response, UdfHistoryResponse::NoData);
    }

    #[test]
    fn history_response_prices_are_descaled_by_pricescale() {
        let bars = vec![bar(0, 123_450, 123_500, 123_400, 123_480, 7)];
        let response = bars_to_history_response(&bars, 0, 60, 100_000);
        match response {
            UdfHistoryResponse::Ok { o, h, l, c, v, t } => {
                assert_eq!(o, vec![1.2345]);
                assert_eq!(h, vec![1.235]);
                assert_eq!(l, vec![1.234]);
                assert_eq!(c, vec![1.2348]);
                assert_eq!(v, vec![7.0]);
                assert_eq!(t, vec![0]);
            }
            UdfHistoryResponse::NoData => panic!("expected Ok"),
        }
    }

    #[test]
    fn response_serializes_to_udfs_real_wire_shape() {
        let response = UdfHistoryResponse::Ok { t: vec![60], o: vec![1.0], h: vec![1.1], l: vec![0.9], c: vec![1.05], v: vec![10.0] };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["s"], "ok");
        assert_eq!(json["t"][0], 60);

        let no_data = serde_json::to_value(UdfHistoryResponse::NoData).unwrap();
        assert_eq!(no_data["s"], "no_data");
    }
}
