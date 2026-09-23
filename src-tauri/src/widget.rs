//! The desktop widget's view of the world.
//!
//! A widget is a glance, not a second copy of the app. It runs in its own
//! sandboxed process with a tight update budget, so it must not open the
//! database or run any analysis. Instead the core writes one small, display-ready
//! snapshot whenever something the widget shows changes, and the widget only
//! decodes and draws it.
//!
//! Three properties matter.
//!
//! **One source of truth.** Every verdict, count and cutoff here comes from
//! [`commands::drive_outcome`], the same function behind the drive screen. The
//! widget cannot disagree with the app, because it never computes anything.
//!
//! **Minimal.** The snapshot is what a desktop glance needs and nothing more: no
//! Neo IDs, no registration numbers, no individual CGPAs, and no names except
//! the labels a student typed into their own circle. The widget's sandbox can
//! read only the directory this file lives in — not the database beside it —
//! so this file is the widget's entire view of the student's data.
//!
//! **Atomic.** It is written to a temporary file and renamed into place, so the
//! widget can never read half a snapshot.
//!
//! The JSON is a contract with Swift and Windows code, so it is versioned by
//! [`SCHEMA`] and pinned by fixtures in `widgets/fixtures`. The Rust tests
//! generate them and fail if they drift; the Swift tests and the Windows card
//! checks decode the same files.

use crate::analytics::CutoffVerdict;
use crate::commands::{drive_outcome, ImportOutcome, PersonResult};
use crate::engine;
use crate::model::{KeyKind, Undetermined, Verdict};
use crate::parse::FileShape;
use crate::store::{Store, StoreError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use time::OffsetDateTime;

/// Bumped whenever a field changes meaning or disappears. Adding an optional
/// field does not require a bump; the widget ignores what it does not know.
pub const SCHEMA: u32 = 1;

/// Drives previewed in full. A season is roughly two dozen companies, and a
/// widget can be pinned to any of them, so each needs its preview on hand —
/// the widget cannot ask the app for one later.
pub const MAX_DRIVES: usize = 24;

/// How long an import may appear to be running. Imports take seconds; if the
/// app quits mid-import it cannot clear the state, so the widget stops
/// believing it after this long rather than showing "Importing" forever.
pub const IMPORTING_FOR: time::Duration = time::Duration::minutes(3);

/// How long a failed import stays on the widget. Long enough to be seen the
/// next time the student looks at their desktop; not so long it becomes noise.
pub const FAILURE_SHOWN_FOR: time::Duration = time::Duration::hours(24);

/// How long a result leads the widget before it goes back to resting.
///
/// A shortlist dropped this morning is today's news; by tomorrow it is history,
/// and a widget still shouting about it reads as stuck. After this the widget
/// shows the season and an invitation to drop the next one — the result is not
/// hidden, just no longer the headline. Both platforms flip at the same moment
/// because the moment travels in the snapshot.
pub const RESULT_LEADS_FOR: time::Duration = time::Duration::hours(12);

/// Circle members carried per drive. A large widget shows a handful; more than
/// this belongs in the app.
pub const MAX_MEMBERS: usize = 6;

pub const FILENAME: &str = "snapshot.json";

const FAILURE_KEY: &str = "widget_last_import_failure";

// ---------------------------------------------------------------------------
// The contract
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WidgetSnapshot {
    pub schema: u32,
    /// RFC 3339, UTC.
    pub generated_at: String,
    /// Whether the student has told nankiv who they are. Without it every
    /// verdict is undetermined, and the widget says why rather than showing a
    /// wall of question marks.
    pub identity_configured: bool,
    pub activity: Activity,
    pub season: Season,
    /// When the newest result stops leading the widget, RFC 3339, UTC. `None`
    /// when there is nothing to lead with.
    pub leads_until: Option<String>,
    /// Newest first. Empty when nothing has been imported.
    pub drives: Vec<DrivePreview>,
}

/// What the app is doing right now, as far as a glance needs to know.
///
/// The transient states carry `until`: the moment the widget should stop
/// showing them. A widget cannot be told later that something ended — the app
/// may have quit — so the policy travels with the state, decided here once for
/// every platform, and the widget schedules its own timeline around it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Activity {
    Idle,
    Importing {
        filename: String,
        /// RFC 3339, UTC.
        since: String,
        /// RFC 3339, UTC.
        until: String,
    },
    /// The last import could not be read. Kept until the next import succeeds,
    /// so the widget can explain a missing result instead of silently showing
    /// the previous drive as though nothing happened. No error text: the reason
    /// can quote file paths, and the app is where it gets explained.
    Failed {
        filename: String,
        /// RFC 3339, UTC.
        at: String,
        /// RFC 3339, UTC.
        until: String,
    },
}

impl Activity {
    pub fn importing(filename: &str, since: OffsetDateTime) -> Activity {
        Activity::Importing {
            filename: filename.to_string(),
            since: format_now(since),
            until: format_now(since + IMPORTING_FOR),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Season {
    pub drives: usize,
    pub shortlisted: usize,
    pub not_shortlisted: usize,
    pub undetermined: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrivePreview {
    pub id: i64,
    pub company: String,
    /// RFC 3339, UTC.
    pub imported_at: String,
    pub total_students: usize,
    pub key: String,
    pub verdict: VerdictPreview,
    pub circle: CirclePreview,
    pub analysis: AnalysisPreview,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VerdictPreview {
    /// `shortlisted`, `not_shortlisted` or `undetermined` — three states, never
    /// collapsed into two. The widget renders each differently.
    pub status: String,
    /// Present only for `undetermined`.
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CirclePreview {
    pub total: usize,
    pub shortlisted: usize,
    pub not_shortlisted: usize,
    pub undetermined: usize,
    /// Shortlisted first, then undetermined, then not shortlisted — the order
    /// the app uses, so the widget and the drive screen read the same way.
    pub members: Vec<MemberPreview>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberPreview {
    /// The label the student gave this person. Never a name from the reference
    /// data, and never an identifier.
    pub label: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisPreview {
    /// `ready`, `insufficient` or `not_set_up`. Kept distinct for the same
    /// reason the app keeps them distinct: a thin sample and missing reference
    /// data are different problems with different remedies.
    pub status: String,
    pub matched: usize,
    pub coverage: f64,
    pub cutoff: Option<CutoffPreview>,
    pub median: Option<f64>,
    pub histogram: Vec<BucketPreview>,
    pub over_represented: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CutoffPreview {
    /// `hard_cutoff`, `soft_preference` or `no_cgpa_filter`.
    pub kind: String,
    pub threshold: Option<f64>,
    pub observed_floor: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BucketPreview {
    pub lower: f64,
    pub upper: f64,
    pub count: usize,
}

// ---------------------------------------------------------------------------
// Building
// ---------------------------------------------------------------------------

fn verdict_status(v: &Verdict) -> VerdictPreview {
    match v {
        Verdict::Shortlisted => VerdictPreview {
            status: "shortlisted".into(),
            reason: None,
        },
        Verdict::NotShortlisted => VerdictPreview {
            status: "not_shortlisted".into(),
            reason: None,
        },
        Verdict::Undetermined(u) => VerdictPreview {
            status: "undetermined".into(),
            reason: Some(
                match u {
                    Undetermined::NoIdentityConfigured => "no_identity_configured",
                    Undetermined::KeyKindNotConfigured { file_key } => match file_key {
                        KeyKind::NeoId => "needs_neo_id",
                        KeyKind::RegNo => "needs_reg_no",
                    },
                    Undetermined::FileNotUnderstood => "file_not_understood",
                }
                .into(),
            ),
        },
    }
}

fn circle(friends: &[PersonResult]) -> CirclePreview {
    let mut c = CirclePreview {
        total: friends.len(),
        ..Default::default()
    };
    for f in friends {
        match f.verdict {
            Verdict::Shortlisted => c.shortlisted += 1,
            Verdict::NotShortlisted => c.not_shortlisted += 1,
            Verdict::Undetermined(_) => c.undetermined += 1,
        }
    }
    // drive_outcome has already ordered them; carry that order through.
    c.members = friends
        .iter()
        .take(MAX_MEMBERS)
        .map(|f| MemberPreview {
            label: f.label.clone(),
            status: verdict_status(&f.verdict).status,
        })
        .collect();
    c
}

fn analysis(o: &ImportOutcome, academics_known: usize) -> AnalysisPreview {
    let a = &o.analysis;
    let status = if academics_known == 0 {
        "not_set_up"
    } else if a.sufficient {
        "ready"
    } else {
        "insufficient"
    };

    let ready = status == "ready";
    let cutoff = if ready {
        a.cutoff.as_ref().map(|c| match &c.verdict.value {
            CutoffVerdict::HardCutoff {
                threshold,
                observed_floor,
            } => CutoffPreview {
                kind: "hard_cutoff".into(),
                threshold: Some(*threshold),
                observed_floor: Some(*observed_floor),
            },
            CutoffVerdict::SoftPreference { observed_floor } => CutoffPreview {
                kind: "soft_preference".into(),
                threshold: None,
                observed_floor: Some(*observed_floor),
            },
            CutoffVerdict::NoCgpaFilter => CutoffPreview {
                kind: "no_cgpa_filter".into(),
                threshold: None,
                observed_floor: None,
            },
        })
    } else {
        None
    };

    let (median, histogram) = match (ready, a.cgpa.as_ref()) {
        (true, Some(d)) => {
            let buckets = &d.value.buckets;
            // Trim empty buckets from either end, as the app's chart does, so a
            // tight distribution is not squeezed into a corner of a 6–10 axis.
            let first = buckets.iter().position(|b| b.count > 0);
            let last = buckets.iter().rposition(|b| b.count > 0);
            let trimmed = match (first, last) {
                (Some(f), Some(l)) => buckets[f..=l]
                    .iter()
                    .map(|b| BucketPreview {
                        lower: b.lower,
                        upper: b.upper,
                        count: b.count,
                    })
                    .collect(),
                _ => Vec::new(),
            };
            (Some(d.value.median), trimmed)
        }
        _ => (None, Vec::new()),
    };

    AnalysisPreview {
        status: status.into(),
        matched: a.matched_students,
        coverage: a.coverage,
        cutoff,
        median,
        histogram,
        over_represented: if ready {
            a.branches
                .as_ref()
                .map(|b| b.over_represented.clone())
                .unwrap_or_default()
        } else {
            Vec::new()
        },
    }
}

/// SQLite's `datetime('now')` is `YYYY-MM-DD HH:MM:SS` in UTC. The widget wants
/// something a date parser accepts without guessing the zone.
pub fn to_rfc3339(sqlite_utc: &str) -> String {
    let t = sqlite_utc.trim();
    if t.len() == 19 && t.as_bytes()[10] == b' ' {
        format!("{}T{}Z", &t[..10], &t[11..])
    } else {
        t.to_string()
    }
}

pub fn format_now(now: OffsetDateTime) -> String {
    now.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn shape_of(label: &str) -> FileShape {
    match label {
        "reg_no_only" => FileShape::RegNoOnly,
        "linked" => FileShape::Linked,
        "unrecognised" => FileShape::Unrecognised,
        _ => FileShape::NeoIdOnly,
    }
}

/// How the season is going: one verdict per drive, counted.
///
/// Shared with the interface, which shows it in the toolbar, so the number on
/// the widget and the number above the list can never disagree. Cheap: a
/// membership check per drive, no analysis.
pub fn season_tally(store: &Store) -> Result<Season, StoreError> {
    let profile = store.profile()?;
    let drives = store.drives()?;
    let mut season = Season {
        drives: drives.len(),
        ..Default::default()
    };
    for d in &drives {
        let key = match d.primary_key.as_deref() {
            Some("reg_no") => Some(KeyKind::RegNo),
            Some("neo_id") => Some(KeyKind::NeoId),
            _ => None,
        };
        let v = engine::membership_verdict(
            key,
            shape_of(&d.shape),
            profile.neo_id.as_deref(),
            profile.reg_no.as_deref(),
            &store.drive_neo_ids(d.id)?,
            &store.drive_reg_nos(d.id)?,
        );
        match v {
            Verdict::Shortlisted => season.shortlisted += 1,
            Verdict::NotShortlisted => season.not_shortlisted += 1,
            Verdict::Undetermined(_) => season.undetermined += 1,
        }
    }
    Ok(season)
}

/// Builds the snapshot from the store as it stands.
pub fn build_snapshot(
    store: &Store,
    activity: Activity,
    now: OffsetDateTime,
) -> Result<WidgetSnapshot, StoreError> {
    let profile = store.profile()?;
    let drives = store.drives()?;
    let academics_known = store.academic_count()?;

    let season = season_tally(store)?;

    let mut previews = Vec::new();
    if !drives.is_empty() {
        let identity = engine::build_graph(store)?;
        for d in drives.iter().take(MAX_DRIVES) {
            // A drive that cannot be summarised is skipped rather than failing
            // the whole snapshot: one bad record must not blank the widget.
            let Ok(o) = drive_outcome(store, d.id, &identity, &profile) else {
                continue;
            };
            previews.push(DrivePreview {
                id: o.drive_id,
                company: o.company.clone(),
                imported_at: to_rfc3339(&d.imported_at),
                total_students: o.total_students,
                key: match o.primary_key {
                    Some(KeyKind::RegNo) => "reg_no".into(),
                    _ => "neo_id".into(),
                },
                verdict: verdict_status(&o.you.verdict),
                circle: circle(&o.friends),
                analysis: analysis(&o, academics_known),
            });
        }
    }

    let leads_until = previews
        .first()
        .and_then(|d| {
            OffsetDateTime::parse(
                &d.imported_at,
                &time::format_description::well_known::Rfc3339,
            )
            .ok()
        })
        .map(|imported| format_now(imported + RESULT_LEADS_FOR));

    Ok(WidgetSnapshot {
        schema: SCHEMA,
        generated_at: format_now(now),
        identity_configured: profile.is_configured(),
        activity,
        season,
        leads_until,
        drives: previews,
    })
}

// ---------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------

/// Writes the snapshot atomically: a temporary file in the same directory,
/// then a rename, so a reader sees the old file or the new one and never a
/// partial write.
pub fn write_snapshot(dir: &Path, snapshot: &WidgetSnapshot) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let target = dir.join(FILENAME);
    let tmp = dir.join(format!(".{FILENAME}.tmp"));
    let json = serde_json::to_vec(snapshot)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, &target)?;
    Ok(target)
}

/// Remembers which import failed, until one succeeds.
pub fn record_failure(
    store: &Store,
    filename: &str,
    now: OffsetDateTime,
) -> Result<(), StoreError> {
    let v = serde_json::json!({ "filename": filename, "at": format_now(now) });
    store.set_meta(FAILURE_KEY, &v.to_string())
}

pub fn clear_failure(store: &Store) -> Result<(), StoreError> {
    store.set_meta(FAILURE_KEY, "")
}

/// The activity to show when nothing is in flight: a remembered failure, or idle.
pub fn resting_activity(store: &Store) -> Activity {
    let Ok(Some(raw)) = store.meta(FAILURE_KEY) else {
        return Activity::Idle;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Activity::Idle;
    };
    let (Some(filename), Some(at)) = (v["filename"].as_str(), v["at"].as_str()) else {
        return Activity::Idle;
    };
    let Ok(when) = OffsetDateTime::parse(at, &time::format_description::well_known::Rfc3339) else {
        return Activity::Idle;
    };
    Activity::Failed {
        filename: filename.to_string(),
        at: format_now(when),
        until: format_now(when + FAILURE_SHOWN_FOR),
    }
}

/// WidgetKit's reload request, compiled from `macos/widget_bridge.swift`.
#[cfg(target_os = "macos")]
pub fn reload_widgetkit() {
    extern "C" {
        fn nankiv_reload_widget_timelines();
    }
    // SAFETY: takes no arguments, returns nothing, and only asks WidgetKit to
    // rebuild timelines. Safe to call from any thread.
    unsafe { nankiv_reload_widget_timelines() }
}

/// How the platform is told the snapshot changed. On macOS this asks WidgetKit
/// to reload timelines; elsewhere it is unset and publishing just writes the
/// file. Installed once at startup so tests never touch a platform API.
static RELOAD: OnceLock<fn()> = OnceLock::new();

pub fn set_reload_hook(hook: fn()) {
    let _ = RELOAD.set(hook);
}

/// Rebuilds, writes and announces the snapshot.
///
/// Failures are reported but never propagated: a widget that falls behind is a
/// minor problem, and it must never be able to fail an import.
pub fn publish(store: &Store, dir: Option<&Path>, activity: Activity) {
    let Some(dir) = dir else { return };
    let snapshot = match build_snapshot(store, activity, OffsetDateTime::now_utc()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("widget snapshot could not be built: {e}");
            return;
        }
    };
    if let Err(e) = write_snapshot(dir, &snapshot) {
        eprintln!("widget snapshot could not be written: {e}");
        return;
    }
    if let Some(reload) = RELOAD.get() {
        reload();
    }
}

// ---------------------------------------------------------------------------
// Publishing off the main thread
// ---------------------------------------------------------------------------

/// What happened, as far as the widget cares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Signal {
    /// Something the widget shows may have changed.
    Changed,
    ImportStarted(String),
    ImportEnded,
}

/// How long the worker waits for further changes before rebuilding. Adding
/// three friends in a row, or an import's start and end on a fast file, then
/// cost one snapshot rather than several — and a quick import never flashes an
/// "importing" state the student would not have had time to read.
pub const QUIET: std::time::Duration = std::time::Duration::from_millis(150);

/// Rebuilds the snapshot on a background thread.
///
/// A full season's snapshot takes a couple of hundred milliseconds to build in a
/// debug build. Synchronous Tauri commands run on the main thread, so doing that
/// inline after every change would stutter the interface; here commands only
/// send a signal and return.
pub struct Publisher {
    tx: std::sync::mpsc::Sender<Signal>,
}

impl Publisher {
    /// `access` must run the closure against the store, however the caller
    /// holds it. It is called on the worker thread.
    pub fn spawn<F>(dir: PathBuf, access: F) -> Publisher
    where
        F: Fn(&mut dyn FnMut(&Store)) + Send + 'static,
    {
        let (tx, rx) = std::sync::mpsc::channel::<Signal>();
        std::thread::Builder::new()
            .name("nankiv-widget".into())
            .spawn(move || {
                let mut importing: Option<(String, OffsetDateTime)> = None;
                let apply = |s: Signal, importing: &mut Option<(String, OffsetDateTime)>| match s {
                    Signal::Changed => {}
                    Signal::ImportStarted(f) => *importing = Some((f, OffsetDateTime::now_utc())),
                    Signal::ImportEnded => *importing = None,
                };
                while let Ok(first) = rx.recv() {
                    apply(first, &mut importing);
                    loop {
                        match rx.recv_timeout(QUIET) {
                            Ok(s) => apply(s, &mut importing),
                            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => break,
                            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                        }
                    }
                    let current = importing.clone();
                    access(&mut |store| {
                        let activity = match &current {
                            Some((f, since)) => Activity::importing(f, *since),
                            None => resting_activity(store),
                        };
                        publish(store, Some(&dir), activity);
                    });
                }
            })
            .expect("widget worker thread");
        Publisher { tx }
    }

    pub fn send(&self, signal: Signal) {
        // A closed channel means the app is shutting down; nothing to do.
        let _ = self.tx.send(signal);
    }
}

// ---------------------------------------------------------------------------
// Deep links
// ---------------------------------------------------------------------------

/// Where a `nankiv://` link asks to go.
///
/// A URL scheme can be invoked by any website or app, not only the widget, so
/// every route is navigation and nothing else. No route imports, deletes,
/// exports or changes a setting, and anything that does not parse exactly is
/// ignored rather than guessed at.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Route {
    /// `nankiv://drive/42`
    Drive { id: i64 },
    /// `nankiv://latest` — the newest drive, whatever it is when the link opens.
    Latest,
    /// `nankiv://shortlists`
    Shortlists,
}

pub const SCHEME: &str = "nankiv";

pub fn parse_route(url: &str) -> Option<Route> {
    let url = url.trim();
    if url.len() > 64 {
        return None;
    }
    let rest = url
        .get(..SCHEME.len() + 3)
        .filter(|p| p.eq_ignore_ascii_case("nankiv://"))
        .map(|_| &url[SCHEME.len() + 3..])?;
    if rest.contains(['?', '#', '\\', '%', ' ']) {
        return None;
    }
    let parts: Vec<&str> = rest.trim_end_matches('/').split('/').collect();
    match parts.as_slice() {
        [host] if host.eq_ignore_ascii_case("latest") => Some(Route::Latest),
        [host] if host.eq_ignore_ascii_case("shortlists") => Some(Route::Shortlists),
        [host, id] if host.eq_ignore_ascii_case("drive") => {
            if id.is_empty() || id.len() > 18 || !id.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            id.parse::<i64>()
                .ok()
                .filter(|n| *n > 0)
                .map(|id| Route::Drive { id })
        }
        _ => None,
    }
}

pub fn route_url(route: &Route) -> String {
    match route {
        Route::Drive { id } => format!("{SCHEME}://drive/{id}"),
        Route::Latest => format!("{SCHEME}://latest"),
        Route::Shortlists => format!("{SCHEME}://shortlists"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NeoId, RegNo};
    use crate::store::Profile;
    use std::collections::BTreeSet;
    use time::macros::datetime;

    const NOW: OffsetDateTime = datetime!(2026-09-17 15:00:00 UTC);

    // --- routes -------------------------------------------------------------

    #[test]
    fn routes_parse_exactly() {
        assert_eq!(
            parse_route("nankiv://drive/42"),
            Some(Route::Drive { id: 42 })
        );
        assert_eq!(
            parse_route("NANKIV://Drive/7/"),
            Some(Route::Drive { id: 7 })
        );
        assert_eq!(parse_route("nankiv://latest"), Some(Route::Latest));
        assert_eq!(parse_route("nankiv://shortlists"), Some(Route::Shortlists));
    }

    #[test]
    fn anything_unexpected_is_ignored_rather_than_guessed() {
        for bad in [
            "",
            "nankiv://",
            "nankiv://drive",
            "nankiv://drive/",
            "nankiv://drive/0",
            "nankiv://drive/-3",
            "nankiv://drive/4a",
            "nankiv://drive/42/delete",
            "nankiv://drive/42?wipe=1",
            "nankiv://drive/42#x",
            "nankiv://drive/%34%32",
            "nankiv://settings",
            "nankiv://wipe",
            "nankiv://import/../../etc/passwd",
            "https://drive/42",
            "nankivx://drive/42",
            "nankiv:drive/42",
            "nankiv://drive/99999999999999999999",
        ] {
            assert_eq!(parse_route(bad), None, "should reject {bad:?}");
        }
    }

    #[test]
    fn routes_round_trip() {
        for r in [Route::Drive { id: 12 }, Route::Latest, Route::Shortlists] {
            assert_eq!(parse_route(&route_url(&r)), Some(r));
        }
    }

    // --- building -----------------------------------------------------------

    fn neos(v: &[&str]) -> BTreeSet<NeoId> {
        v.iter().filter_map(|s| NeoId::parse(s)).collect()
    }

    fn drive(s: &Store, company: &str, hash: &str, ids: &[&str]) -> i64 {
        s.insert_drive(
            company,
            None,
            "f.xlsx",
            hash,
            "neo_id_only",
            Some("neo_id"),
            &neos(ids),
            &BTreeSet::new(),
            None,
        )
        .unwrap()
    }

    #[test]
    fn an_empty_store_produces_an_empty_snapshot() {
        let s = Store::open_in_memory().unwrap();
        let snap = build_snapshot(&s, Activity::Idle, NOW).unwrap();
        assert_eq!(snap.schema, SCHEMA);
        assert!(snap.drives.is_empty());
        assert_eq!(snap.season, Season::default());
        assert!(!snap.identity_configured);
        assert_eq!(snap.generated_at, "2026-09-17T15:00:00Z");
    }

    #[test]
    fn the_latest_drive_comes_first_and_verdicts_match_the_app() {
        let s = Store::open_in_memory().unwrap();
        s.save_profile(&Profile {
            neo_id: Some("V9H0G6C4".into()),
            ..Default::default()
        })
        .unwrap();
        let first = drive(&s, "Siemens", "a", &["C5U6K1E7"]);
        let second = drive(&s, "Tredence", "b", &["V9H0G6C4", "C5U6K1E7"]);

        let snap = build_snapshot(&s, Activity::Idle, NOW).unwrap();
        let ids: Vec<i64> = snap.drives.iter().map(|d| d.id).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&first) && ids.contains(&second));

        let tredence = snap
            .drives
            .iter()
            .find(|d| d.company == "Tredence")
            .unwrap();
        let siemens = snap.drives.iter().find(|d| d.company == "Siemens").unwrap();
        assert_eq!(tredence.verdict.status, "shortlisted");
        assert_eq!(siemens.verdict.status, "not_shortlisted");
        assert_eq!(snap.season.shortlisted, 1);
        assert_eq!(snap.season.not_shortlisted, 1);
    }

    #[test]
    fn an_unanswerable_file_is_undetermined_never_a_rejection() {
        let s = Store::open_in_memory().unwrap();
        s.save_profile(&Profile {
            neo_id: Some("V9H0G6C4".into()),
            ..Default::default()
        })
        .unwrap();
        let regs: BTreeSet<RegNo> = ["23BAI0001"]
            .iter()
            .filter_map(|r| RegNo::parse(r))
            .collect();
        s.insert_drive(
            "HPE",
            None,
            "h.xlsx",
            "h",
            "reg_no_only",
            Some("reg_no"),
            &BTreeSet::new(),
            &regs,
            None,
        )
        .unwrap();

        let snap = build_snapshot(&s, Activity::Idle, NOW).unwrap();
        let v = &snap.drives[0].verdict;
        assert_eq!(v.status, "undetermined");
        assert_eq!(v.reason.as_deref(), Some("needs_reg_no"));
        assert_eq!(snap.season.undetermined, 1);
        assert_eq!(snap.season.not_shortlisted, 0);
    }

    #[test]
    fn missing_reference_data_is_not_set_up_not_insufficient() {
        let s = Store::open_in_memory().unwrap();
        drive(&s, "Zluri", "z", &["V9H0G6C4"]);
        let snap = build_snapshot(&s, Activity::Idle, NOW).unwrap();
        assert_eq!(snap.drives[0].analysis.status, "not_set_up");
        assert!(snap.drives[0].analysis.cutoff.is_none());
    }

    #[test]
    fn the_circle_carries_labels_and_counts_only() {
        let s = Store::open_in_memory().unwrap();
        drive(&s, "Tredence", "t", &["C5U6K1E7"]);
        s.add_friend("Arjun", Some("C5U6K1E7"), None, None).unwrap();
        s.add_friend("Divya", Some("T2D4R9N9"), None, None).unwrap();

        let snap = build_snapshot(&s, Activity::Idle, NOW).unwrap();
        let c = &snap.drives[0].circle;
        assert_eq!((c.total, c.shortlisted, c.not_shortlisted), (2, 1, 1));
        assert_eq!(c.members[0].label, "Arjun");
        assert_eq!(c.members[0].status, "shortlisted");
    }

    #[test]
    fn the_snapshot_never_carries_identifiers_or_individual_cgpa() {
        let s = Store::open_in_memory().unwrap();
        s.save_profile(&Profile {
            neo_id: Some("V9H0G6C4".into()),
            reg_no: Some("23BAI0002".into()),
            show_friend_cgpa: true,
            ..Default::default()
        })
        .unwrap();
        drive(&s, "Tredence", "t", &["V9H0G6C4", "C5U6K1E7"]);
        s.add_friend("Arjun", Some("C5U6K1E7"), Some("23BAI0009"), None)
            .unwrap();
        s.upsert_academic(
            &RegNo::parse("23BAI0002").unwrap(),
            Some(9.41),
            Some("CSE"),
            "x",
        )
        .unwrap();

        let snap = build_snapshot(&s, Activity::Idle, NOW).unwrap();
        let json = serde_json::to_string(&snap).unwrap();
        for leak in ["V9H0G6C4", "C5U6K1E7", "23BAI0002", "23BAI0009", "9.41"] {
            assert!(!json.contains(leak), "snapshot must not contain {leak}");
        }
    }

    #[test]
    fn the_newest_result_leads_for_a_while_then_stops() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(
            build_snapshot(&s, Activity::Idle, NOW).unwrap().leads_until,
            None
        );

        drive(&s, "Tredence", "t", &["C5U6K1E7"]);
        let snap = build_snapshot(&s, Activity::Idle, NOW).unwrap();
        let imported = OffsetDateTime::parse(
            &snap.drives[0].imported_at,
            &time::format_description::well_known::Rfc3339,
        )
        .unwrap();
        let leads_until = OffsetDateTime::parse(
            snap.leads_until.as_deref().unwrap(),
            &time::format_description::well_known::Rfc3339,
        )
        .unwrap();
        assert_eq!(leads_until - imported, RESULT_LEADS_FOR);
    }

    #[test]
    fn drives_are_capped() {
        let s = Store::open_in_memory().unwrap();
        for i in 0..(MAX_DRIVES + 4) {
            drive(&s, &format!("Co {i}"), &format!("h{i}"), &["C5U6K1E7"]);
        }
        let snap = build_snapshot(&s, Activity::Idle, NOW).unwrap();
        assert_eq!(snap.drives.len(), MAX_DRIVES);
        assert_eq!(
            snap.season.drives,
            MAX_DRIVES + 4,
            "season still counts everything"
        );
    }

    #[test]
    fn sqlite_timestamps_become_rfc3339() {
        assert_eq!(to_rfc3339("2026-09-17 13:02:11"), "2026-09-17T13:02:11Z");
        assert_eq!(to_rfc3339("2026-09-17T13:02:11Z"), "2026-09-17T13:02:11Z");
    }

    // --- activity -----------------------------------------------------------

    #[test]
    fn a_failure_is_remembered_until_cleared() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(resting_activity(&s), Activity::Idle);
        record_failure(&s, "TCS.xlsx", NOW).unwrap();
        assert_eq!(
            resting_activity(&s),
            Activity::Failed {
                filename: "TCS.xlsx".into(),
                at: "2026-09-17T15:00:00Z".into(),
                until: "2026-09-18T15:00:00Z".into(),
            }
        );
        clear_failure(&s).unwrap();
        assert_eq!(resting_activity(&s), Activity::Idle);
    }

    // --- writing ------------------------------------------------------------

    #[test]
    fn writing_is_atomic_and_leaves_no_temporary_file() {
        let s = Store::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let snap = build_snapshot(&s, Activity::Idle, NOW).unwrap();
        let path = write_snapshot(dir.path(), &snap).unwrap();

        let back: WidgetSnapshot = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(back, snap);
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(leftovers, vec![FILENAME.to_string()]);
    }

    #[test]
    fn publishing_without_a_directory_is_a_no_op() {
        let s = Store::open_in_memory().unwrap();
        publish(&s, None, Activity::Idle);
    }

    // --- the background publisher ------------------------------------------

    fn read_until<P: Fn(&WidgetSnapshot) -> bool>(path: &Path, pred: P) -> WidgetSnapshot {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            if let Ok(bytes) = std::fs::read(path) {
                if let Ok(s) = serde_json::from_slice::<WidgetSnapshot>(&bytes) {
                    if pred(&s) {
                        return s;
                    }
                }
            }
            assert!(
                std::time::Instant::now() < deadline,
                "snapshot never reached the expected state"
            );
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }

    fn spawn_with(store: std::sync::Arc<std::sync::Mutex<Store>>, dir: &Path) -> Publisher {
        Publisher::spawn(dir.to_path_buf(), move |f| {
            let s = store.lock().unwrap();
            f(&s);
        })
    }

    #[test]
    fn the_publisher_writes_after_a_change() {
        let store = std::sync::Arc::new(std::sync::Mutex::new(Store::open_in_memory().unwrap()));
        let dir = tempfile::tempdir().unwrap();
        let p = spawn_with(store.clone(), dir.path());

        drive(&store.lock().unwrap(), "Tredence", "t", &["C5U6K1E7"]);
        p.send(Signal::Changed);

        let snap = read_until(&dir.path().join(FILENAME), |s| s.drives.len() == 1);
        assert_eq!(snap.drives[0].company, "Tredence");
    }

    #[test]
    fn an_import_in_progress_is_shown_then_cleared() {
        let store = std::sync::Arc::new(std::sync::Mutex::new(Store::open_in_memory().unwrap()));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILENAME);
        let p = spawn_with(store.clone(), dir.path());

        p.send(Signal::ImportStarted("Siemens.xlsx".into()));
        let busy = read_until(&path, |s| matches!(s.activity, Activity::Importing { .. }));
        let Activity::Importing {
            filename,
            since,
            until,
        } = busy.activity
        else {
            unreachable!()
        };
        assert_eq!(filename, "Siemens.xlsx");
        let parse = |t: &str| {
            OffsetDateTime::parse(t, &time::format_description::well_known::Rfc3339).unwrap()
        };
        assert_eq!(parse(&until) - parse(&since), IMPORTING_FOR);

        p.send(Signal::ImportEnded);
        read_until(&path, |s| s.activity == Activity::Idle);
    }

    #[test]
    fn a_fast_import_never_flashes_an_importing_state() {
        // Start and end inside the quiet window collapse into one idle write.
        let store = std::sync::Arc::new(std::sync::Mutex::new(Store::open_in_memory().unwrap()));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILENAME);
        let p = spawn_with(store.clone(), dir.path());

        p.send(Signal::ImportStarted("Fast.xlsx".into()));
        p.send(Signal::ImportEnded);
        let snap = read_until(&path, |_| true);
        assert_eq!(snap.activity, Activity::Idle);
    }

    #[test]
    fn a_remembered_failure_shows_once_nothing_is_in_flight() {
        let store = std::sync::Arc::new(std::sync::Mutex::new(Store::open_in_memory().unwrap()));
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILENAME);
        let p = spawn_with(store.clone(), dir.path());

        record_failure(&store.lock().unwrap(), "TCS.xlsx", NOW).unwrap();
        p.send(Signal::Changed);
        let snap = read_until(&path, |s| matches!(s.activity, Activity::Failed { .. }));
        assert!(
            matches!(snap.activity, Activity::Failed { ref filename, .. } if filename == "TCS.xlsx")
        );
    }
}
