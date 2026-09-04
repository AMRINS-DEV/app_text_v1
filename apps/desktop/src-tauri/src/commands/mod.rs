//! Tauri commands invoked from the dashboard's system page (§4, §11.2:
//! POST /api/system/install/mt5-bridge). Phase 4/11 scope.

#[tauri::command]
pub fn install_bridge() -> Result<(), String> {
    Err("install_bridge is Phase 4/11 scope".into())
}

#[tauri::command]
pub fn service_ctl(_service: String, _action: String) -> Result<(), String> {
    Err("service_ctl is Phase 4/11 scope".into())
}

#[tauri::command]
pub fn rotate_keys() -> Result<(), String> {
    Err("rotate_keys is Phase 4/11 scope".into())
}
