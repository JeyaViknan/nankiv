//! nankiv core.
//!
//! All parsing, matching, statistics and storage live here. The interface layer
//! receives only what it renders, so personal data never crosses into the
//! webview unaggregated.

pub mod analytics;
pub mod commands;
pub mod engine;
pub mod identity;
pub mod model;
pub mod parse;
pub mod store;

use commands::AppState;
use std::sync::Mutex;
use tauri::Manager;

/// Starts the application.
///
/// The database lives in the platform application-data directory and never
/// leaves the machine. There is no network client in this binary.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app
                .path()
                .app_data_dir()
                .expect("platform application-data directory");
            std::fs::create_dir_all(&dir)?;
            let store = store::Store::open(&dir.join("nankiv.db"))
                .map_err(|e| format!("could not open the local database: {e}"))?;
            app.manage(AppState {
                store: Mutex::new(store),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_profile,
            commands::save_profile,
            commands::list_friends,
            commands::add_friend,
            commands::remove_friend,
            commands::import_shortlist,
            commands::import_reference,
            commands::list_drives,
            commands::delete_drive,
            commands::get_drive_detail,
            commands::lookup_identifier,
            commands::search_students,
            commands::compare_rounds,
            commands::identity_stats,
            commands::data_inventory,
            commands::wipe_all_data,
            commands::share_summary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running nankiv");
}
