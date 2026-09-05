//! Supervises core/gateway/agent processes with health display and log
//! streaming (§4, §13). Phase 4/11 scope.

pub enum ManagedService {
    Core,
    Gateway,
    Agents,
}

pub struct ProcessManager;

impl ProcessManager {
    pub fn start(&mut self, _service: ManagedService) -> Result<(), String> {
        Err("process supervision is Phase 4/11 scope".into())
    }
}
