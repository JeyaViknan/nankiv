//! nankiv core.
//!
//! All parsing, matching, statistics and storage live here. The interface layer
//! receives only what it renders, so personal data never crosses into the
//! webview unaggregated.

pub mod analytics;
pub mod commands;
pub mod desktop;
pub mod engine;
pub mod export;
pub mod identity;
pub mod model;
pub mod parse;
pub mod reference;
pub mod store;
pub mod widget;

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

            // The widget reads only this directory — its sandbox is scoped to it —
            // so the snapshot lives beside the database, never inside it.
            #[cfg(target_os = "macos")]
            widget::set_reload_hook(widget::reload_widgetkit);
            let handle = app.handle().clone();
            let publisher = widget::Publisher::spawn(dir.join("widget"), move |f| {
                if let Some(state) = handle.try_state::<AppState>() {
                    if let Ok(s) = state.store.lock() {
                        f(&s);
                    }
                }
            });

            // A link or a file passed on the command line is how Windows and
            // Linux hand either to an app they are launching.
            let arguments: Vec<String> = std::env::args().skip(1).collect();
            let launch_route = arguments.iter().find_map(|a| widget::parse_route(a));
            let launch_files: Vec<String> = arguments
                .iter()
                .filter_map(|a| desktop::spreadsheet(a))
                .map(|p| p.to_string_lossy().into_owned())
                .collect();

            app.manage(AppState {
                store: Mutex::new(store),
                widget: Some(publisher),
                pending_route: Mutex::new(launch_route),
                pending_files: Mutex::new(launch_files),
            });

            // Publish once at launch, so the widget reflects anything that
            // changed while the app was closed — including a new app version
            // whose analysis differs.
            if let Some(state) = app.try_state::<AppState>() {
                state.widget_changed();
            }

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
            commands::export_shortlist,
            commands::take_pending_route,
            commands::take_pending_files,
            commands::stage_dropped_file,
        ])
        .build(tauri::generate_context!())
        .expect("error while building nankiv")
        .run(|app, event| {
            // macOS delivers `nankiv://` links — from the widget, or from
            // anywhere else — as an open-URLs event rather than as arguments.
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            if let tauri::RunEvent::Opened { urls } = &event {
                if let Some(route) = urls
                    .iter()
                    .rev()
                    .find_map(|u| widget::parse_route(u.as_str()))
                {
                    open_route(app, route);
                }
                let files: Vec<String> = urls
                    .iter()
                    .filter_map(|u| desktop::spreadsheet(u.as_str()))
                    .map(|p| p.to_string_lossy().into_owned())
                    .collect();
                if !files.is_empty() {
                    open_files(app, files);
                }
            }
            let _ = (app, event);
        });
}

/// Brings the window forward and hands a route to the interface.
///
/// The route is both stored and emitted. If the interface is already running it
/// acts on the event and then clears the stored copy; if the link launched the
/// app, nothing is listening yet, and the interface collects the stored copy
/// once it has loaded. Opening the same drive twice is harmless, so the overlap
/// needs no coordination.
/// Hands the interface shortlists the desktop opened with nankiv.
///
/// The same two paths as a link: acted on immediately if the interface is
/// running, held for it to collect if the drop is what launched the app.
#[cfg_attr(not(any(target_os = "macos", target_os = "ios")), allow(dead_code))]
fn open_files(app: &tauri::AppHandle, files: Vec<String>) {
    use tauri::Emitter;
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut pending) = state.pending_files.lock() {
            pending.clone_from(&files);
        }
    }
    let _ = app.emit("open-files", &files);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(not(any(target_os = "macos", target_os = "ios")), allow(dead_code))]
fn open_route(app: &tauri::AppHandle, route: widget::Route) {
    use tauri::Emitter;
    if let Some(state) = app.try_state::<AppState>() {
        if let Ok(mut pending) = state.pending_route.lock() {
            *pending = Some(route.clone());
        }
    }
    let _ = app.emit("deep-link", &route);
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
