//! TradingView adapter (§5.4, §17 Phase 8 scope: "TradingView webhook + UDF
//! datafeed"). TradingView cannot execute orders for retail accounts (§5.4's
//! own note), so this crate only ever implements `MarketDataSource` — there
//! is no legitimate `Broker` implementation here. Two real, if narrowly
//! scoped, pieces, both served from one `axum` HTTP server (the first Rust
//! HTTP server in this project — chosen because it is already tokio-based
//! like every other adapter here, and nothing larger already exists in the
//! workspace to reuse):
//!
//! - `POST /webhook`: a Pine-alert receiver (`webhook` module) that turns
//!   an authenticated alert into a `Tick` for its named symbol — an alert
//!   fires *at* a price, so that price is a genuine, if one-sided, market
//!   observation (§5.4 point 2).
//! - `GET /udf/{config,symbols,history}`: a real TradingView UDF datafeed
//!   server (`udf` module) backed by this project's own
//!   `market_data::BarAggregator`, so an actual TradingView chart (or any
//!   UDF-speaking tool) can point at this project's data — "UDF datafeed"
//!   is the literal shape §17's Phase 8 roadmap row names.
//!
//! Every webhook alert also feeds the same in-memory bar store the UDF
//! server reads from and the trait's own `history()` method reads from —
//! one real data path serving three different consumers, not three
//! independent stubs.

pub mod udf;
pub mod webhook;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use crossbeam_channel::{bounded, Receiver, Sender};
use domain::ports::*;
use domain::{Bar, SymbolId, Tick};
use market_data::BarAggregator;
use tokio::sync::oneshot;

use udf::{bars_to_history_response, resample, resolution_to_timeframe_seconds, UdfConfig, UdfHistoryResponse, UdfSymbolInfo};
use webhook::{alert_to_tick, authenticate, WebhookAlert};

/// Webhook alerts are folded into 1-minute bars internally; coarser UDF
/// resolutions are served by resampling this native resolution up (`udf::resample`).
const NATIVE_TIMEFRAME_SECONDS: u32 = 60;

pub struct TradingViewConfig {
    pub bind_addr: SocketAddr,
    pub webhook_token: String,
    /// Ticker <-> `SymbolId` table. Pine alerts identify instruments by
    /// ticker string, which the shared `SymbolSpec` (`symbol_id` +
    /// `price_digits`) does not carry, so this adapter needs its own table
    /// rather than deriving one from `subscribe`'s argument.
    pub symbols: Vec<(String, SymbolId)>,
    /// Fixed-point scale matching `Tick`'s convention (e.g. `100_000` for a
    /// 5-decimal FX quote).
    pub price_scale: i64,
}

struct BarState {
    aggregator: BarAggregator,
    history: Vec<Bar>,
}

#[derive(Clone)]
struct SharedState {
    expected_token: Arc<String>,
    symbol_by_ticker: Arc<HashMap<String, SymbolId>>,
    price_scale: i64,
    bars: Arc<Mutex<HashMap<SymbolId, BarState>>>,
    tick_tx: Sender<Tick>,
}

pub struct TradingViewMarketData {
    config: TradingViewConfig,
    rx: Option<Receiver<Tick>>,
    bars: Arc<Mutex<HashMap<SymbolId, BarState>>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    #[allow(dead_code)]
    handle: Option<std::thread::JoinHandle<()>>,
}

impl TradingViewMarketData {
    pub fn new(config: TradingViewConfig) -> Self {
        Self { config, rx: None, bars: Arc::new(Mutex::new(HashMap::new())), shutdown_tx: None, handle: None }
    }
}

impl MarketDataSource for TradingViewMarketData {
    fn subscribe(&mut self, _symbols: &[SymbolSpec]) -> Result<()> {
        // This adapter's ticker<->SymbolId table comes from
        // `TradingViewConfig.symbols` (set at construction), not from this
        // method's argument — see the field's own doc comment.
        let (tick_tx, tick_rx) = bounded(1024);
        self.rx = Some(tick_rx);

        let symbol_by_ticker: HashMap<String, SymbolId> = self.config.symbols.iter().cloned().collect();
        let state = SharedState {
            expected_token: Arc::new(self.config.webhook_token.clone()),
            symbol_by_ticker: Arc::new(symbol_by_ticker),
            price_scale: self.config.price_scale,
            bars: self.bars.clone(),
            tick_tx,
        };

        let bind_addr = self.config.bind_addr;
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        self.shutdown_tx = Some(shutdown_tx);

        let handle = std::thread::Builder::new()
            .name("tradingview-http".into())
            .spawn(move || {
                let rt = tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                    .expect("tokio runtime");
                rt.block_on(async move {
                    let app = build_router(state);
                    let listener = match tokio::net::TcpListener::bind(bind_addr).await {
                        Ok(l) => l,
                        Err(_) => return,
                    };
                    let _ = axum::serve(listener, app).with_graceful_shutdown(async { let _ = shutdown_rx.await; }).await;
                });
            })
            .expect("failed to spawn tradingview-http thread");

        self.handle = Some(handle);
        Ok(())
    }

    fn poll_tick(&mut self) -> Option<Tick> {
        self.rx.as_ref()?.try_recv().ok()
    }

    fn history(&self, sym: SymbolId, tf: Timeframe, from_ns: u64, to_ns: u64) -> Result<Vec<Bar>> {
        let target_seconds = timeframe_to_seconds(tf);
        let map = self.bars.lock().unwrap();
        let Some(bar_state) = map.get(&sym) else {
            return Ok(Vec::new());
        };
        let resampled = resample(&bar_state.history, target_seconds);
        Ok(resampled.into_iter().filter(|b| b.ts_open_ns >= from_ns && b.ts_open_ns <= to_ns).collect())
    }

    fn capabilities(&self) -> FeedCaps {
        FeedCaps { depth: false, volume: false, ticks: true }
    }
}

impl Drop for TradingViewMarketData {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

fn timeframe_to_seconds(tf: Timeframe) -> u32 {
    match tf {
        Timeframe::M1 => 60,
        Timeframe::M5 => 300,
        Timeframe::M15 => 900,
        Timeframe::H1 => 3_600,
        Timeframe::H4 => 14_400,
        Timeframe::D1 => 86_400,
    }
}

fn build_router(state: SharedState) -> Router {
    Router::new()
        .route("/webhook", post(handle_webhook))
        .route("/udf/config", get(handle_udf_config))
        .route("/udf/symbols", get(handle_udf_symbols))
        .route("/udf/history", get(handle_udf_history))
        .with_state(state)
}

async fn handle_webhook(State(state): State<SharedState>, Json(alert): Json<WebhookAlert>) -> impl IntoResponse {
    if authenticate(&alert, &state.expected_token).is_err() {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    let Some(&symbol_id) = state.symbol_by_ticker.get(&alert.ticker) else {
        return (StatusCode::BAD_REQUEST, format!("unknown ticker: {}", alert.ticker)).into_response();
    };
    let tick = alert_to_tick(&alert, symbol_id, state.price_scale);

    {
        let mut bars = state.bars.lock().unwrap();
        let entry = bars
            .entry(symbol_id)
            .or_insert_with(|| BarState { aggregator: BarAggregator::new(symbol_id, NATIVE_TIMEFRAME_SECONDS), history: Vec::new() });
        if let Some(closed) = entry.aggregator.on_tick(&tick) {
            entry.history.push(closed);
        }
    }

    let _ = state.tick_tx.send(tick);
    StatusCode::OK.into_response()
}

async fn handle_udf_config() -> impl IntoResponse {
    Json(UdfConfig::default())
}

#[derive(serde::Deserialize)]
struct SymbolsQuery {
    symbol: String,
}

async fn handle_udf_symbols(State(state): State<SharedState>, Query(q): Query<SymbolsQuery>) -> impl IntoResponse {
    if !state.symbol_by_ticker.contains_key(&q.symbol) {
        return (StatusCode::NOT_FOUND, "unknown_symbol").into_response();
    }
    Json(UdfSymbolInfo {
        name: q.symbol.clone(),
        ticker: q.symbol.clone(),
        // This project's own instruments are FX/CFD-shaped (§5.4's own
        // MT5-primary framing) — a real multi-asset UDF server would derive
        // this per symbol rather than hardcoding it, out of Phase 8 scope.
        kind: "forex".into(),
        session: "24x7".into(),
        timezone: "Etc/UTC".into(),
        exchange: "TRADEOS".into(),
        minmov: 1,
        pricescale: state.price_scale,
        has_intraday: true,
        supported_resolutions: UdfConfig::default().supported_resolutions,
    })
    .into_response()
}

#[derive(serde::Deserialize)]
struct HistoryQuery {
    symbol: String,
    resolution: String,
    from: i64,
    to: i64,
}

async fn handle_udf_history(State(state): State<SharedState>, Query(q): Query<HistoryQuery>) -> impl IntoResponse {
    let Some(&symbol_id) = state.symbol_by_ticker.get(&q.symbol) else {
        return Json(UdfHistoryResponse::NoData).into_response();
    };
    let Some(target_seconds) = resolution_to_timeframe_seconds(&q.resolution) else {
        return (StatusCode::BAD_REQUEST, "unsupported resolution").into_response();
    };
    let from_ns = (q.from.max(0) as u64) * 1_000_000_000;
    let to_ns = (q.to.max(0) as u64) * 1_000_000_000;

    let history = {
        let bars = state.bars.lock().unwrap();
        bars.get(&symbol_id).map(|b| b.history.clone()).unwrap_or_default()
    };
    let resampled = resample(&history, target_seconds);
    Json(bars_to_history_response(&resampled, from_ns, to_ns, state.price_scale)).into_response()
}
