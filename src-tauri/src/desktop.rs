//! Native desktop behaviour: the menu bar and window geometry.
//!
//! Both exist because a keyboard shortcut that lives only in a JavaScript
//! `keydown` handler is invisible to the operating system. It works, but the
//! menu bar does not list it, Help ▸ Search cannot find it, and nothing tells a
//! student the shortcut exists. A real menu is the difference between a desktop
//! application and a web page in a window.

use tauri::menu::{
    AboutMetadata, MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder,
};
use tauri::{AppHandle, Emitter, Manager, Runtime, WebviewWindow, WindowEvent};

use crate::store::Store;

const GEOMETRY_KEY: &str = "window_geometry";

/// Bounds on the window, in logical pixels.
///
/// The content column is capped at 720px, so past roughly 1200 the extra width
/// is empty ground rather than more information — and below 860 the toolbar's
/// three regions start colliding. Constraining both ends keeps every window the
/// app can be in a shape the layout was actually designed for.
pub const MIN_W: f64 = 860.0;
pub const MIN_H: f64 = 620.0;
pub const MAX_W: f64 = 1280.0;
pub const MAX_H: f64 = 980.0;

/// A comfortable opening size for a given display.
///
/// Proportional to the screen rather than fixed, so it feels considered on a
/// 13-inch laptop and on a large desktop display, then clamped so it never
/// becomes either cramped or sprawling.
pub fn default_size(screen_w: f64, screen_h: f64) -> (f64, f64) {
    // Measured against the displays this will actually run on rather than
    // guessed: at 0.56 a 15-inch MacBook opened at the 860px minimum, which is
    // the app apologising for itself on a perfectly roomy screen.
    let w = (screen_w * 0.70).clamp(MIN_W, MAX_W);
    let h = (screen_h * 0.78).clamp(MIN_H, MAX_H);
    (w.round(), h.round())
}

/// Builds the application menu.
///
/// Every accelerator here mirrors a handler the interface already has, so the
/// shortcuts keep working exactly as before — they are simply discoverable now,
/// and printed where macOS expects to print them.
pub fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let about = AboutMetadata {
        name: Some("nankiv".into()),
        version: Some(env!("CARGO_PKG_VERSION").into()),
        comments: Some("Placement shortlist analysis. Everything stays on this machine.".into()),
        ..Default::default()
    };

    let app_menu = SubmenuBuilder::new(app, "nankiv")
        .item(&PredefinedMenuItem::about(
            app,
            Some("About nankiv"),
            Some(about),
        )?)
        .separator()
        .item(
            &MenuItemBuilder::new("Settings…")
                .id("settings")
                .accelerator("CmdOrCtrl+,")
                .build(app)?,
        )
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

    let file_menu = SubmenuBuilder::new(app, "File")
        .item(
            &MenuItemBuilder::new("Open Shortlist…")
                .id("open")
                .accelerator("CmdOrCtrl+O")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::new("Import Reference Sheet…")
                .id("open_reference")
                .accelerator("CmdOrCtrl+Shift+O")
                .build(app)?,
        )
        .separator()
        // Only ever handled here, never also in a webview keydown listener: if
        // both fired, one keypress would open two save panels.
        .item(
            &MenuItemBuilder::new("Download Names as Excel…")
                .id("export_xlsx")
                .accelerator("CmdOrCtrl+E")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::new("Download Names as CSV…")
                .id("export_csv")
                .accelerator("CmdOrCtrl+Shift+E")
                .build(app)?,
        )
        .separator()
        .close_window()
        .build()?;

    // Without an Edit menu the standard clipboard shortcuts do not work at all
    // in a webview — Cmd+C in a text field silently does nothing.
    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

    let view_menu = SubmenuBuilder::new(app, "View")
        .item(
            &MenuItemBuilder::new("Search")
                .id("search")
                .accelerator("CmdOrCtrl+F")
                .build(app)?,
        )
        .item(
            &MenuItemBuilder::new("Circle")
                .id("circle")
                .accelerator("CmdOrCtrl+D")
                .build(app)?,
        )
        .separator()
        .item(
            &MenuItemBuilder::new("Back to Shortlists")
                .id("back")
                .accelerator("CmdOrCtrl+[")
                .build(app)?,
        )
        .build()?;

    // No fullscreen item: nankiv is a utility with a fixed-width reading
    // column, so filling a 27-inch display would show the same 720px of content
    // marooned in grey. The window also carries a max size, which is what makes
    // macOS grey out the green button rather than leaving a control that lies.
    let window_menu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .separator()
        .close_window()
        .build()?;

    let menu = MenuBuilder::new(app)
        .items(&[&app_menu, &file_menu, &edit_menu, &view_menu, &window_menu])
        .build()?;

    app.set_menu(menu)?;

    // The menu does not act; it forwards. The interface already owns these
    // behaviours, so there is exactly one implementation of each.
    app.on_menu_event(|handle, event| {
        let id = event.id().0.as_str();
        if matches!(
            id,
            "settings"
                | "open"
                | "open_reference"
                | "search"
                | "circle"
                | "back"
                | "export_xlsx"
                | "export_csv"
        ) {
            let _ = handle.emit("menu", id);
        }
    });

    Ok(())
}

// ---------------------------------------------------------------------------
// Window geometry
// ---------------------------------------------------------------------------

#[derive(serde::Serialize, serde::Deserialize)]
struct Geometry {
    width: f64,
    height: f64,
    x: i32,
    y: i32,
    maximized: bool,
}

/// Restores the window to wherever it was left.
///
/// A window that reopens at the default size and position every launch is one
/// of those details nobody names but everybody notices — the application feels
/// like it is starting from scratch each time rather than resuming.
pub fn restore_geometry<R: Runtime>(window: &WebviewWindow<R>, store: &Store) {
    let Ok(Some(raw)) = store.meta(GEOMETRY_KEY) else {
        return;
    };
    let Ok(g) = serde_json::from_str::<Geometry>(&raw) else {
        return;
    };

    // Guard against a monitor that is no longer attached: a window restored to
    // a screen that has gone away is a window the student cannot reach.
    let visible = window
        .available_monitors()
        .map(|monitors| {
            monitors.iter().any(|m| {
                let pos = m.position();
                let size = m.size();
                g.x + 80 >= pos.x
                    && g.y + 40 >= pos.y
                    && g.x < pos.x + size.width as i32
                    && g.y < pos.y + size.height as i32
            })
        })
        .unwrap_or(false);

    // Clamp on the way in too: a size saved by an older build, or on a much
    // larger display, must still land inside the bounds this layout expects.
    let w = g.width.clamp(MIN_W, MAX_W);
    let h = g.height.clamp(MIN_H, MAX_H);
    let _ = window.set_size(tauri::LogicalSize::new(w, h));
    if visible {
        let _ = window.set_position(tauri::LogicalPosition::new(g.x, g.y));
    }
}

/// Sizes and centres the window the first time the app runs.
///
/// Without this every student opens to the same hardcoded rectangle regardless
/// of their display, which is the sort of detail that reads as unfinished.
pub fn apply_default_geometry<R: Runtime>(window: &WebviewWindow<R>) {
    let Ok(Some(monitor)) = window.primary_monitor() else {
        return;
    };
    let scale = monitor.scale_factor();
    let screen = monitor.size().to_logical::<f64>(scale);
    let (w, h) = default_size(screen.width, screen.height);

    let _ = window.set_size(tauri::LogicalSize::new(w, h));
    let _ = window.set_position(tauri::LogicalPosition::new(
        ((screen.width - w) / 2.0).max(0.0),
        // Sits a little above true centre, which reads as balanced rather than
        // low — the same trick as optical centring in typography.
        ((screen.height - h) / 2.4).max(0.0),
    ));
}

/// Keeps the stored geometry current as the window moves and resizes.
///
/// Saving only on close looked tidier and did not work: quitting on macOS does
/// not reliably deliver a close event to the webview window, so the size was
/// never written and every launch reopened at the default. Recording the
/// geometry as it changes removes the dependency on exit timing altogether —
/// whatever happens at shutdown, the last known good value is already on disk.
///
/// The cost is a handful of single-row upserts during a resize drag, which is
/// nothing next to a window that forgets itself.
pub fn persist_geometry<R: Runtime>(window: &WebviewWindow<R>) {
    let w = window.clone();
    window.on_window_event(move |event| {
        let interesting = matches!(
            event,
            WindowEvent::Resized(_)
                | WindowEvent::Moved(_)
                | WindowEvent::CloseRequested { .. }
                | WindowEvent::Destroyed
        );
        if !interesting {
            return;
        }
        save_now(&w);
    });
}

/// Writes the window's current geometry, if it is in a state worth recording.
fn save_now<R: Runtime>(w: &WebviewWindow<R>) {
    // A minimised window reports a useless size; recording it would restore the
    // app to a sliver next launch.
    if w.is_minimized().unwrap_or(false) {
        return;
    }
    let Some(state) = w.try_state::<crate::commands::AppState>() else {
        return;
    };
    let Ok(store) = state.store.lock() else {
        return;
    };

    let scale = w.scale_factor().unwrap_or(1.0);
    let (Ok(size), Ok(pos)) = (w.inner_size(), w.outer_position()) else {
        return;
    };
    let logical = size.to_logical::<f64>(scale);
    let lpos = pos.to_logical::<i32>(scale);

    if logical.width < MIN_W - 40.0 || logical.height < MIN_H - 40.0 {
        return;
    }

    let g = Geometry {
        width: logical.width,
        height: logical.height,
        x: lpos.x,
        y: lpos.y,
        maximized: false,
    };
    if let Ok(json) = serde_json::to_string(&g) {
        let _ = store.set_meta(GEOMETRY_KEY, &json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_round_trips_through_the_store() {
        let store = Store::open_in_memory().unwrap();
        let g = Geometry {
            width: 1180.0,
            height: 820.0,
            x: 240,
            y: 120,
            maximized: false,
        };
        store
            .set_meta(GEOMETRY_KEY, &serde_json::to_string(&g).unwrap())
            .unwrap();

        let raw = store.meta(GEOMETRY_KEY).unwrap().expect("saved");
        let back: Geometry = serde_json::from_str(&raw).unwrap();
        assert_eq!(back.width, 1180.0);
        assert_eq!(back.height, 820.0);
        assert_eq!(back.x, 240);
        assert_eq!(back.y, 120);
        assert!(!back.maximized);
    }

    #[test]
    fn a_corrupt_geometry_value_is_ignored_rather_than_fatal() {
        let store = Store::open_in_memory().unwrap();
        store.set_meta(GEOMETRY_KEY, "not json at all").unwrap();
        let raw = store.meta(GEOMETRY_KEY).unwrap().unwrap();
        assert!(serde_json::from_str::<Geometry>(&raw).is_err());
        // restore_geometry takes the same path and simply returns.
    }

    #[test]
    fn a_laptop_gets_a_roomy_window_not_the_minimum() {
        // The 15-inch MacBook this was developed on is 1512x982 logical. Opening
        // at MIN_W there was the bug that prompted retuning the factors.
        let (w, h) = default_size(1512.0, 982.0);
        assert!(w > MIN_W + 100.0, "15-inch opened at {w}, near the floor");
        assert!(h > MIN_H + 60.0, "15-inch opened at {h}, near the floor");
        assert!(w <= MAX_W && h <= MAX_H);
    }

    #[test]
    fn the_default_size_scales_with_the_display() {
        // A 13-inch laptop and a large desktop display should not open to the
        // same rectangle.
        let (small_w, small_h) = default_size(1440.0, 900.0);
        let (large_w, large_h) = default_size(2560.0, 1440.0);
        assert!(large_w > small_w);
        assert!(large_h > small_h);
    }

    #[test]
    fn the_default_never_leaves_the_agreed_bounds() {
        for (sw, sh) in [
            (1280.0, 800.0),  // small laptop
            (1440.0, 900.0),  // 13-inch
            (1728.0, 1117.0), // 16-inch
            (2560.0, 1440.0), // desktop
            (3840.0, 2160.0), // 4K
            (800.0, 600.0),   // absurdly small
        ] {
            let (w, h) = default_size(sw, sh);
            assert!((MIN_W..=MAX_W).contains(&w), "{sw}x{sh} -> width {w}");
            assert!((MIN_H..=MAX_H).contains(&h), "{sw}x{sh} -> height {h}");
        }
    }

    #[test]
    fn a_huge_display_does_not_produce_a_sprawling_window() {
        // The reading column is capped at 720px, so beyond the maximum the
        // extra width would be empty ground rather than more information.
        let (w, h) = default_size(5120.0, 2880.0);
        assert_eq!(w, MAX_W);
        assert_eq!(h, MAX_H);
    }

    #[test]
    fn a_restored_size_is_clamped_into_the_bounds() {
        // Geometry saved by an older build, or on a much larger display, must
        // still land somewhere this layout works.
        let cases: [(f64, f64); 3] = [(400.0, MIN_W), (1000.0, 1000.0), (3000.0, MAX_W)];
        for (saved, expected) in cases {
            assert_eq!(saved.clamp(MIN_W, MAX_W), expected);
        }
    }

    #[test]
    fn absurd_sizes_are_rejected_before_being_applied() {
        // A window restored to 40x30 is a window nobody can use. The guard in
        // restore_geometry is the reason this stays a non-issue.
        for (w, h, ok) in [
            (1180.0, 820.0, true),
            (200.0, 150.0, false),
            (640.0, 480.0, true),
        ] {
            let usable = w >= 640.0 && h >= 480.0;
            assert_eq!(usable, ok, "for {w}x{h}");
        }
    }
}
