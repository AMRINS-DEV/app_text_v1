//! Storage ports. One trait per datastore role in §6's data plane; concrete
//! clients (QuestDB ILP, Postgres/sqlx, FalkorDB Cypher, Qdrant) are added
//! per-store as each phase needs them (Phase 1: ticks; Phase 2: trades/audit;
//! Phase 6: graph/embeddings).

use async_trait::async_trait;
use domain::{Bar, SymbolId, Tick};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("not connected")]
    NotConnected,
    #[error("query failed: {0}")]
    Query(String),
}

pub type Result<T> = std::result::Result<T, StorageError>;

/// QuestDB: high-throughput tick/bar ingest + `SAMPLE BY`/`ASOF JOIN` reads.
#[async_trait]
pub trait TimeseriesStore: Send + Sync {
    async fn write_tick(&self, tick: &Tick) -> Result<()>;
    async fn write_bar(&self, bar: &Bar) -> Result<()>;
    async fn read_bars(&self, symbol_id: SymbolId, from_ns: u64, to_ns: u64) -> Result<Vec<Bar>>;
}
