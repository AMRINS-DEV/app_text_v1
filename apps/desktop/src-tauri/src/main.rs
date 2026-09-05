// TradeOS desktop shell (§13, §4). Terminal-free install/control, OS
// keychain access, ~10MB vs Electron's ~150MB. GUI build requires
// webkit2gtk (Linux) / WebView2 (Windows) — not available in this
// environment, so this binary is structural only for Phase 0 (see
// README.md's "What is real vs. stubbed").
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod keychain;
mod mt5_locator;
mod process_manager;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::install_bridge,
            commands::service_ctl,
            commands::rotate_keys,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
