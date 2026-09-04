//! OS keyring master key management (§13's envelope encryption: OS keychain
//! -> master key -> AES-256-GCM data keys in Postgres). Phase 4 scope.

pub struct Keychain;

impl Keychain {
    pub fn load_master_key() -> Result<Vec<u8>, String> {
        Err("keychain access is Phase 4 scope".into())
    }
}
