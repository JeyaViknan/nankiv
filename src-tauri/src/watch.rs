//! Watching Downloads for shortlists — opt-in, local, and only while nankiv
//! is open.
//!
//! Turned on in Settings, it looks at the top level of the Downloads folder
//! every few seconds for spreadsheets that arrived since it was turned on. A
//! file is read once its size has stopped changing, so a download in progress
//! is never parsed half-written. Anything that is not a usable shortlist — a
//! timetable, a CGPA sheet, a list already imported — is passed over without
//! a word. A shortlist is imported exactly as a dropped file is, and the
//! interface is told so it can say so.
//!
//! Nothing else is read, nothing leaves the machine, and nothing runs when
//! the app is closed: there is no background agent.

use crate::commands::{self, AppState};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tauri::{Emitter, Manager};

/// Present while watching is on: when it was turned on, RFC 3339. Files from
/// before then are never imported by the watcher.
pub const SINCE_KEY: &str = "watch_downloads_since";

const POLL: Duration = Duration::from_secs(4);

/// Starts the watcher. It idles until the setting is on.
pub fn spawn(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut watcher = Watcher::default();
        loop {
            std::thread::sleep(POLL);
            watcher.tick(&app);
        }
    });
}

#[derive(Default)]
struct Watcher {
    /// Size and time a file had when first seen, to tell when it has settled.
    sizes: HashMap<PathBuf, (SystemTime, u64)>,
    /// Files already looked at, as they were: never parsed twice.
    done: HashSet<(PathBuf, SystemTime)>,
}

impl Watcher {
    fn tick(&mut self, app: &tauri::AppHandle) {
        let Some(state) = app.try_state::<AppState>() else {
            return;
        };
        let since = state
            .store
            .lock()
            .ok()
            .and_then(|s| s.meta(SINCE_KEY).ok().flatten())
            .and_then(|v| {
                time::OffsetDateTime::parse(&v, &time::format_description::well_known::Rfc3339).ok()
            });
        let Some(since) = since else {
            self.sizes.clear();
            return;
        };
        let Ok(dir) = app.path().download_dir() else {
            return;
        };

        for path in self.settled(&dir, since.into()) {
            if let Some(Ok(outcome)) = commands::import_watched(&state, &path) {
                let _ = app.emit("watched-import", &outcome);
                // One bounce if nankiv is behind another window; the system
                // ignores it when nankiv is already in front.
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window
                        .request_user_attention(Some(tauri::UserAttentionType::Informational));
                }
            }
        }
    }

    /// New spreadsheets whose size has not changed since the last look.
    fn settled(&mut self, dir: &Path, since: SystemTime) -> Vec<PathBuf> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut ready = Vec::new();
        for path in entries.filter_map(|e| e.ok()).map(|e| e.path()) {
            if !is_candidate(&path) {
                continue;
            }
            let Some((modified, len)) = std::fs::metadata(&path)
                .ok()
                .and_then(|m| Some((m.modified().ok()?, m.len())))
            else {
                continue;
            };
            if modified < since || self.done.contains(&(path.clone(), modified)) {
                continue;
            }
            match self.sizes.insert(path.clone(), (modified, len)) {
                Some(before) if before == (modified, len) => {
                    self.sizes.remove(&path);
                    self.done.insert((path.clone(), modified));
                    ready.push(path);
                }
                _ => {} // first sighting, or still being written
            }
        }
        ready
    }
}

/// A spreadsheet a person saved, not a lock file or a download in progress.
fn is_candidate(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy())
        .unwrap_or_default();
    !name.starts_with('.')
        && !name.starts_with("~$")
        && crate::desktop::spreadsheet(&path.to_string_lossy()).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downloads_are_not_watched_until_asked() {
        // Opt-in: a fresh install, every migration applied, has never been
        // told to look in Downloads.
        let s = crate::store::Store::open_in_memory().unwrap();
        assert_eq!(s.meta(SINCE_KEY).unwrap(), None);
    }

    #[test]
    fn only_spreadsheets_a_person_saved_are_candidates() {
        for ok in ["Siemens shortlist.xlsx", "list.csv", "a.xls"] {
            assert!(is_candidate(Path::new(ok)), "{ok}");
        }
        for not in [
            "~$Siemens shortlist.xlsx",
            ".hidden.xlsx",
            "Siemens shortlist.xlsx.crdownload",
            "Siemens.xlsx.download",
            "notes.pdf",
        ] {
            assert!(!is_candidate(Path::new(not)), "{not}");
        }
    }

    #[test]
    fn a_file_is_read_only_once_it_has_settled_and_only_once() {
        let dir = tempfile::tempdir().unwrap();
        let since = SystemTime::now() - Duration::from_secs(60);
        let file = dir.path().join("Zluri shortlist.xlsx");
        let mut w = Watcher::default();

        std::fs::write(&file, b"part").unwrap();
        assert!(w.settled(dir.path(), since).is_empty(), "first sighting");
        std::fs::write(&file, b"partial, still arriving").unwrap();
        assert!(w.settled(dir.path(), since).is_empty(), "size changed");
        assert_eq!(w.settled(dir.path(), since), vec![file.clone()], "settled");
        assert!(w.settled(dir.path(), since).is_empty(), "never twice");
    }

    #[test]
    fn files_from_before_watching_began_are_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("old.xlsx"), b"x").unwrap();
        let since = SystemTime::now() + Duration::from_secs(60);
        let mut w = Watcher::default();
        assert!(w.settled(dir.path(), since).is_empty());
        assert!(w.settled(dir.path(), since).is_empty());
    }
}
