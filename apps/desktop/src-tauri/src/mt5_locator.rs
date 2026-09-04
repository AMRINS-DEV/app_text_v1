//! Locates the MT5 terminal path and deploys the bridge EA (§4, §5.4).
//! Phase 1/11 scope — depends on `bridge/mt5`'s EA existing to deploy.

pub struct Mt5Installation {
    pub terminal_path: std::path::PathBuf,
    pub data_path: std::path::PathBuf,
}

pub fn locate() -> Result<Mt5Installation, String> {
    Err("mt5_locator is Phase 1/11 scope".into())
}
