//! nankiv core.
//!
//! All parsing, matching, statistics and storage live here. The interface layer
//! receives only what it renders, so personal data never crosses into the
//! webview unaggregated.

pub mod analytics;
pub mod commands;
pub mod desktop;
pub mod engine;
pub mod identity;
pub mod model;
pub mod parse;
pub mod reference;
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

            // nankiv ships knowing the cohort, so a student gets an analysis on
            // their first import rather than being asked to supply a sheet the
            // maintainer already has.
            match reference::seed(&store) {
                Ok(r) if !r.already_seeded && r.students > 0 => {
                    println!(
                        "seeded reference data: {} students, {} academic records",
                        r.students, r.academics
                    );
                }
                Err(e) => eprintln!("reference seeding failed, continuing: {e}"),
                _ => {}
            }
            // Geometry has to be read before the store moves into managed state.
            if let Some(window) = app.get_webview_window("main") {
                // A remembered size wins; a first run gets one chosen for the
                // display it is actually on.
                if store.meta("window_geometry").ok().flatten().is_some() {
                    desktop::restore_geometry(&window, &store);
                } else {
                    desktop::apply_default_geometry(&window);
                }
            }

            app.manage(AppState {
                store: Mutex::new(store),
            });

            if let Some(window) = app.get_webview_window("main") {
                desktop::persist_geometry(&window);
            }

            if let Err(e) = desktop::build_menu(app.handle()) {
                eprintln!("could not build the application menu: {e}");
            }

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
            commands::restore_drive,
            commands::rename_drive,
            commands::set_drive_round,
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
