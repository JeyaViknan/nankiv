//! Tauri command layer.
//!
//! This is the only boundary the interface can reach through, and it is
//! deliberately narrow. Commands return view models that have already been
//! resolved and aggregated — the webview never receives a full student table,
//! so personal data does not leave the Rust side except as something being
//! rendered right now.

use crate::analytics::DriveAnalysis;
use crate::engine::{self, ResolvedIdentity};
use crate::identity::matcher::MatchOutcome;
use crate::model::*;
use crate::parse::{self, FileShape};
use crate::store::{DriveRecord, DriveSnapshot, Friend, Profile, Store, StoreError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub store: Mutex<Store>,
    /// Absent in tests and wherever the widget directory is unavailable.
    pub widget: Option<crate::widget::Publisher>,
    /// A `nankiv://` link that arrived before the interface was listening,
    /// typically the one that launched the app from a widget.
    pub pending_route: Mutex<Option<crate::widget::Route>>,
    /// Shortlists the desktop handed us — dropped on the app icon, or opened
    /// with nankiv — before the interface was listening.
    pub pending_files: Mutex<Vec<String>>,
}

impl AppState {
    /// Tells the widget something it shows may have changed. Cheap: it only
    /// sends a signal; the snapshot is rebuilt on a background thread.
    pub fn widget_changed(&self) {
        self.signal(crate::widget::Signal::Changed);
    }

    pub fn signal(&self, s: crate::widget::Signal) {
        if let Some(w) = &self.widget {
            w.send(s);
        }
    }
}

/// Errors crossing the IPC boundary, in language a student can act on.
#[derive(Debug, Serialize)]
pub struct CommandError {
    pub code: String,
    pub message: String,
    /// Extra context, e.g. the header text of a file we could not read.
    pub detail: Option<String>,
}

impl CommandError {
    pub(crate) fn new(code: &str, message: impl Into<String>) -> Self {
        CommandError {
            code: code.into(),
            message: message.into(),
            detail: None,
        }
    }
    pub(crate) fn with_detail(mut self, d: impl Into<String>) -> Self {
        self.detail = Some(d.into());
        self
    }
}

impl From<StoreError> for CommandError {
    fn from(e: StoreError) -> Self {
        match e {
            StoreError::DuplicateDrive { drive_id, company } => CommandError {
                code: "duplicate_drive".into(),
                message: format!("You already imported this {company} shortlist."),
                detail: Some(drive_id.to_string()),
            },
            other => CommandError::new("storage", other.to_string()),
        }
    }
}

type R<T> = Result<T, CommandError>;

fn store<'a>(state: &'a AppState) -> std::sync::MutexGuard<'a, Store> {
    state.store.lock().expect("store lock poisoned")
}

// ---------------------------------------------------------------------------
// View models
// ---------------------------------------------------------------------------

/// One person's status on a shortlist, safe to render.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonResult {
    pub label: String,
    pub neo_id: Option<String>,
    pub reg_no: Option<String>,
    pub verdict: Verdict,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportOutcome {
    pub drive_id: i64,
    pub company: String,
    pub drive_date: Option<String>,
    pub total_students: usize,
    pub shape: FileShape,
    pub primary_key: Option<KeyKind>,
    pub you: PersonResult,
    /// The CGPA the student entered for themselves, for their own place on
    /// the distribution. Nobody else's CGPA is ever sent to the interface.
    pub your_cgpa: Option<f64>,
    /// Where your answer came from, for "Why does it think I'm in?".
    pub evidence: engine::Evidence,
    /// This drive's rounds, when it is one of several.
    pub progression: Option<engine::Progression>,
    pub friends: Vec<PersonResult>,
    pub analysis: DriveAnalysis,
    /// What this file taught the identity graph.
    pub learned_verified: usize,
    pub learned_named: usize,
    /// Present when the file could not be understood at all.
    pub unreadable_headers: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IdentityStats {
    pub students_known: usize,
    pub names_known: usize,
    pub academics_known: usize,
    pub edges: usize,
    pub conflicts: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SearchResult {
    Found {
        neo_id: String,
        name: String,
        confidence: String,
    },
    Ambiguous {
        candidates: Vec<SearchCandidate>,
    },
    NotFound,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchCandidate {
    pub neo_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoundDiff {
    pub from_company: String,
    pub to_company: String,
    pub advanced: usize,
    pub dropped: usize,
    pub added: usize,
    pub advanced_friends: Vec<String>,
    pub dropped_friends: Vec<String>,
}

// ---------------------------------------------------------------------------
// Profile and friends
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_profile(state: tauri::State<AppState>) -> R<Profile> {
    Ok(store(&state).profile()?)
}

#[tauri::command]
pub fn save_profile(state: tauri::State<AppState>, profile: Profile) -> R<()> {
    // Validate before storing, so a typo does not silently make every future
    // verdict undeterminable.
    if let Some(n) = profile.neo_id.as_deref().filter(|s| !s.trim().is_empty()) {
        if NeoId::parse(n).is_none() {
            return Err(CommandError::new(
                "bad_neo_id",
                "That's not a Neo ID. It's eight characters, like V9H0G6C4.",
            ));
        }
    }
    if let Some(c) = profile.cgpa {
        if sanitise_cgpa(c).is_none() {
            return Err(CommandError::new(
                "bad_cgpa",
                "A CGPA is out of 10 — like 8.42.",
            ));
        }
    }
    if let Some(r) = profile.reg_no.as_deref().filter(|s| !s.trim().is_empty()) {
        if RegNo::parse(r).is_none() {
            return Err(CommandError::new(
                "bad_reg_no",
                "That doesn't look like a registration number — like 23BAI0002.",
            ));
        }
    }
    let cleaned = Profile {
        neo_id: profile
            .neo_id
            .as_deref()
            .and_then(NeoId::parse)
            .map(|x| x.as_str().to_string()),
        reg_no: profile
            .reg_no
            .as_deref()
            .and_then(RegNo::parse)
            .map(|x| x.as_str().to_string()),
        ..profile
    };
    store(&state).save_profile(&cleaned)?;
    state.widget_changed();
    Ok(())
}

#[tauri::command]
pub fn list_friends(state: tauri::State<AppState>) -> R<Vec<Friend>> {
    Ok(store(&state).friends()?)
}

#[tauri::command]
pub fn add_friend(
    state: tauri::State<AppState>,
    label: String,
    neo_id: Option<String>,
    reg_no: Option<String>,
    group_tag: Option<String>,
) -> R<i64> {
    let neo = neo_id.as_deref().and_then(NeoId::parse);
    let reg = reg_no.as_deref().and_then(RegNo::parse);
    if neo.is_none() && reg.is_none() {
        return Err(CommandError::new(
            "no_identifier",
            "Add a Neo ID or registration number. A name alone can't be checked.",
        ));
    }
    let s = store(&state);
    let id = s.add_friend(
        label.trim(),
        neo.as_ref().map(|x| x.as_str()),
        reg.as_ref().map(|x| x.as_str()),
        group_tag.as_deref(),
    )?;
    state.widget_changed();
    Ok(id)
}

#[tauri::command]
pub fn remove_friend(state: tauri::State<AppState>, id: i64) -> R<()> {
    store(&state).remove_friend(id)?;
    state.widget_changed();
    Ok(())
}

// ---------------------------------------------------------------------------
// Import
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn import_shortlist(
    state: tauri::State<AppState>,
    path: String,
    company_override: Option<String>,
    replace_existing: Option<bool>,
) -> R<ImportOutcome> {
    let filename = PathBuf::from(&path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "shortlist.xlsx".into());
    signalled_import(&state, &filename, || {
        import_shortlist_inner(state.clone(), path, company_override, replace_existing)
    })
}

/// Runs an import with the widget told it is happening, and how it ended.
fn signalled_import(
    state: &tauri::State<AppState>,
    filename: &str,
    import: impl FnOnce() -> R<ImportOutcome>,
) -> R<ImportOutcome> {
    use crate::widget::{self, Signal};

    state.signal(Signal::ImportStarted(filename.to_string()));
    let result = import();

    // Remember a genuine failure so the widget can explain a missing result,
    // and forget it as soon as any import goes through. A duplicate or a
    // reference sheet is not a failure: the file was a reasonable thing to
    // drop, it just did not add a new drive.
    {
        let s = store(state);
        let now = time::OffsetDateTime::now_utc();
        let _ = match &result {
            Ok(_) => widget::clear_failure(&s),
            Err(e) if matches!(e.code.as_str(), "duplicate_drive" | "imported_reference") => {
                widget::clear_failure(&s)
            }
            Err(_) => widget::record_failure(&s, filename, now),
        };
    }
    state.signal(Signal::ImportEnded);
    result
}

/// Imports a shortlist the Downloads watcher found, or `None` for a file that
/// is not one to import: unreadable, not a shortlist, a reference sheet, or a
/// list already imported. Those are passed over without a word — the student
/// did not hand nankiv this file, so a failure is not theirs to read about.
pub(crate) fn import_watched(
    state: &tauri::State<AppState>,
    path: &std::path::Path,
) -> Option<R<ImportOutcome>> {
    let parsed = parse::parse_file(path).ok()?;
    if !parsed.shape.is_usable() {
        return None;
    }
    let academic = crate::parse::academic::parse_academic_sheet(path).unwrap_or_default();
    if academic.iter().any(|r| r.cgpa.is_some()) {
        return None;
    }
    if store(state)
        .find_by_hash(&parsed.content_hash)
        .ok()?
        .is_some()
    {
        return None;
    }
    let filename = path.file_name()?.to_string_lossy().into_owned();
    Some(signalled_import(state, &filename, || {
        record_shortlist(state, &parsed, &filename, None, None)
    }))
}

/// Whether Downloads is being watched for shortlists.
#[tauri::command]
pub fn watch_downloads(state: tauri::State<AppState>) -> R<bool> {
    Ok(store(&state).meta(crate::watch::SINCE_KEY)?.is_some())
}

/// Turns watching Downloads on or off. Files already there when it is turned
/// on are left alone: only what arrives afterwards is checked.
#[tauri::command]
pub fn set_watch_downloads(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    on: bool,
) -> R<()> {
    use tauri::Manager;
    let s = store(&state);
    if !on {
        s.clear_meta(crate::watch::SINCE_KEY)?;
        return Ok(());
    }
    // Look now, so the system's permission prompt follows the switch, and a
    // refusal is reported here rather than failing quietly later.
    let dir = app.path().download_dir().map_err(|_| {
        CommandError::new(
            "no_downloads",
            "nankiv couldn't find your Downloads folder.",
        )
    })?;
    std::fs::read_dir(&dir).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            CommandError::new(
                "downloads_denied",
                "No access to Downloads. Allow it in System Settings → Privacy & Security → Files and Folders.",
            )
        } else {
            CommandError::new("no_downloads", format!("Couldn't read Downloads — {e}"))
        }
    })?;
    let now = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| CommandError::new("clock", e.to_string()))?;
    s.set_meta(crate::watch::SINCE_KEY, &now)?;
    Ok(())
}

/// What a paste would import, shown before it does.
#[derive(Debug, Clone, Serialize)]
pub struct PastePreview {
    pub neo_ids: usize,
    pub reg_nos: usize,
    /// Lines of words with no identifier on them, which will be skipped.
    pub unread_lines: usize,
    /// A name read from the words around the identifiers, if any named one.
    pub company: Option<String>,
}

/// Reads a pasted list without storing anything, so the student can see what
/// was found and name the drive before it is checked.
#[tauri::command]
pub fn preview_paste(text: String) -> R<PastePreview> {
    let pasted = read_paste(&text)?;
    let named = crate::naming::infer("", &pasted.parsed.titles, &[]);
    Ok(PastePreview {
        neo_ids: pasted.parsed.neo_ids.len(),
        reg_nos: pasted.parsed.reg_nos.len(),
        unread_lines: pasted.unread_lines,
        company: (named.company != crate::naming::UNNAMED).then_some(named.company),
    })
}

/// Checks a pasted list of identifiers exactly as a file is checked.
#[tauri::command]
pub fn import_pasted(
    state: tauri::State<AppState>,
    text: String,
    company: Option<String>,
) -> R<ImportOutcome> {
    let filename = parse::text::PASTED_SHEET;
    signalled_import(&state, filename, || {
        let pasted = read_paste(&text)?;
        record_shortlist(&state, &pasted.parsed, filename, company, None)
    })
}

fn read_paste(text: &str) -> R<parse::text::PastedText> {
    parse::text::parse_text(text).map_err(|e| match e {
        parse::ParseError::Empty => CommandError::new(
            "nothing_to_paste",
            "No Neo IDs or registration numbers in what you pasted.",
        ),
        parse::ParseError::TooLarge(_) => CommandError::new(
            "paste_too_large",
            "Too much text to paste. Import it as a spreadsheet instead.",
        ),
        other => CommandError::new("parse_failed", other.to_string()),
    })
}

/// A spreadsheet in the Downloads folder, offered for import.
#[derive(Debug, Clone, Serialize)]
pub struct RecentDownload {
    pub path: String,
    pub name: String,
    /// RFC 3339.
    pub modified: String,
    /// A drive already came from a file of this name.
    pub imported: bool,
}

/// How far back "recent" reaches, and how many are offered. The point is
/// the shortlist that has only just arrived — not a browse through the
/// fortnight's downloads, which the open panel already does better.
const RECENT_WITHIN: std::time::Duration = std::time::Duration::from_secs(60 * 60);
const RECENT_LIMIT: usize = 2;

/// The newest few, from the last hour, newest first.
fn just_downloaded(
    mut found: Vec<(std::time::SystemTime, PathBuf)>,
    now: std::time::SystemTime,
) -> Vec<(std::time::SystemTime, PathBuf)> {
    let cutoff = now - RECENT_WITHIN;
    found.retain(|(modified, _)| *modified >= cutoff);
    found.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
    found.truncate(RECENT_LIMIT);
    found
}

/// The newest spreadsheets in the Downloads folder.
///
/// Read only when asked: on a Mac the first look prompts for access to
/// Downloads, and that prompt should follow the student's click, never
/// appear unbidden at launch.
#[tauri::command]
pub fn recent_downloads(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
) -> R<Vec<RecentDownload>> {
    use tauri::Manager;
    let dir = app.path().download_dir().map_err(|_| {
        CommandError::new(
            "no_downloads",
            "nankiv couldn't find your Downloads folder.",
        )
    })?;
    let entries = std::fs::read_dir(&dir).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            CommandError::new(
                "downloads_denied",
                "No access to Downloads. Allow it in System Settings → Privacy & Security → Files and Folders.",
            )
        } else {
            CommandError::new("no_downloads", format!("Couldn't read Downloads — {e}"))
        }
    })?;

    let imported: std::collections::BTreeSet<String> = store(&state)
        .drives()?
        .into_iter()
        .map(|d| d.source_filename)
        .collect();
    let found: Vec<(std::time::SystemTime, PathBuf)> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| crate::desktop::spreadsheet(&p.to_string_lossy()).is_some())
        .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
        .collect();

    Ok(just_downloaded(found, std::time::SystemTime::now())
        .into_iter()
        .map(|(modified, path)| {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            RecentDownload {
                imported: imported.contains(&name),
                path: path.to_string_lossy().into_owned(),
                modified: time::OffsetDateTime::from(modified)
                    .format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default(),
                name,
            }
        })
        .collect())
}

fn import_shortlist_inner(
    state: tauri::State<AppState>,
    path: String,
    company_override: Option<String>,
    replace_existing: Option<bool>,
) -> R<ImportOutcome> {
    let p = PathBuf::from(&path);
    let filename = p
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "shortlist.xlsx".into());

    let parsed = parse::parse_file(&p)
        .map_err(|e| CommandError::new("parse_failed", format!("Couldn't read that file — {e}")))?;

    let s = store(&state);

    // One drop target has to work for both kinds of spreadsheet a student
    // receives. Without this, dropping the batch CGPA sheet — which is keyed by
    // registration number — silently created a nonsense "drive" of 2,500
    // students, while the reference import stayed hidden behind a button in
    // Settings that nobody found.
    //
    // The test is a CGPA column: shortlists never carry one, reference sheets
    // always do. A file with both is recorded as a drive *and* mined for
    // academics, because it genuinely is both.
    let academic_rows = crate::parse::academic::parse_academic_sheet(&p).unwrap_or_default();
    if academic_rows.iter().any(|r| r.cgpa.is_some()) {
        let learned = engine::ingest_academics(&s, &academic_rows, &filename)?;
        if parsed.neo_ids.is_empty() {
            return Err(CommandError::new(
                "imported_reference",
                format!("Reference sheet added: CGPA for {learned} students"),
            )
            .with_detail(filename));
        }
    }

    drop(s);
    record_shortlist(
        &state,
        &parsed,
        &filename,
        company_override,
        replace_existing,
    )
}

/// Records a parsed shortlist and answers it. Files and pasted lists both end
/// here, so a list is checked, named, linked to its earlier rounds and
/// answered the same way however it arrived.
fn record_shortlist(
    state: &tauri::State<AppState>,
    parsed: &parse::ParsedFile,
    filename: &str,
    company_override: Option<String>,
    replace_existing: Option<bool>,
) -> R<ImportOutcome> {
    let s = store(state);

    // A file we cannot key on tells us nothing. Report it as such rather than
    // recording an empty drive that would read as a rejection.
    if !parsed.shape.is_usable() {
        let headers = parsed.observed_headers.clone();
        return Err(CommandError::new(
            "file_not_understood",
            "No Neo IDs or registration numbers to match. Nothing was saved.",
        )
        .with_detail(if headers.is_empty() {
            "No recognisable column was found.".to_string()
        } else {
            format!("Columns found: {}", headers.join(", "))
        }));
    }

    let named = crate::naming::infer(filename, &parsed.titles, &parsed.sheet_names);
    let (company, drive_date) = match &company_override {
        Some(c) if !c.trim().is_empty() => (c.trim().to_string(), None),
        _ => (named.company, named.date),
    };

    if replace_existing.unwrap_or(false) {
        if let Some(existing) = s.find_by_hash(&parsed.content_hash)? {
            s.delete_drive(existing.id)?;
        }
    }

    let drive_id = match s.insert_drive(
        &company,
        drive_date.as_deref(),
        filename,
        &parsed.content_hash,
        shape_label(parsed.shape),
        parsed.primary_key.map(key_label),
        &parsed.neo_ids,
        &parsed.reg_nos,
        named.round.as_deref(),
    ) {
        // The same list again. If it was first imported before positions were
        // kept, it has them now, and "Why does it think I'm in?" can show the
        // file. Positions already kept are left as they were.
        Err(crate::store::StoreError::DuplicateDrive { drive_id, company }) => {
            s.record_origins(drive_id, &member_origins(parsed))?;
            return Err(crate::store::StoreError::DuplicateDrive { drive_id, company }.into());
        }
        other => other?,
    };
    s.record_origins(drive_id, &member_origins(parsed))?;

    // Harvest identity links before resolving, so this file improves its own
    // analysis as well as every future one.
    let harvest = engine::harvest_identity(&s, parsed, filename)?;
    let identity = engine::build_graph(&s)?;
    engine::link_round(&s, drive_id)?;

    let profile = s.profile()?;
    let mut outcome = drive_outcome(&s, drive_id, &identity, &profile)?;
    outcome.learned_verified = harvest.verified_links;
    outcome.learned_named = harvest.named_links;
    Ok(outcome)
}

/// Where each identifier in a parsed file sat, ready to store.
fn member_origins(parsed: &parse::ParsedFile) -> Vec<crate::store::MemberOrigin> {
    let mut out = Vec::new();
    for (row, origin) in parsed.rows.iter().zip(&parsed.origins) {
        let ids = [
            (
                row.neo_id.as_ref().map(|n| n.as_str()),
                "neo_id",
                &origin.neo_column,
            ),
            (
                row.reg_no.as_ref().map(|r| r.as_str()),
                "reg_no",
                &origin.reg_column,
            ),
        ];
        for (value, kind, column) in ids {
            if let Some(value) = value {
                out.push(crate::store::MemberOrigin {
                    kind: kind.to_string(),
                    value: value.to_string(),
                    sheet: origin.sheet.clone(),
                    row: origin.row,
                    column: column.as_ref().map(|c| c.letter.clone()),
                    header: column.as_ref().and_then(|c| c.header.clone()),
                });
            }
        }
    }
    out
}

fn resolve_person(
    label: &str,
    neo: Option<&str>,
    reg: Option<&str>,
    parsed: &parse::ParsedFile,
    identity: &ResolvedIdentity,
) -> R<PersonResult> {
    let verdict = engine::membership_verdict(
        parsed.primary_key,
        parsed.shape,
        neo,
        reg,
        &parsed.neo_ids,
        &parsed.reg_nos,
    );
    let resolved = identity.resolve(neo, reg);

    Ok(PersonResult {
        label: label.to_string(),
        neo_id: neo.map(|s| s.to_string()),
        reg_no: reg.map(|s| s.to_string()),
        verdict,
        confidence: resolved.confidence.label().to_string(),
    })
}

fn shape_label(s: FileShape) -> &'static str {
    match s {
        FileShape::NeoIdOnly => "neo_id_only",
        FileShape::RegNoOnly => "reg_no_only",
        FileShape::Linked => "linked",
        FileShape::Unrecognised => "unrecognised",
    }
}

fn key_label(k: KeyKind) -> &'static str {
    match k {
        KeyKind::NeoId => "neo_id",
        KeyKind::RegNo => "reg_no",
    }
}

// ---------------------------------------------------------------------------
// History, search, comparison
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn list_drives(state: tauri::State<AppState>) -> R<Vec<DriveListItem>> {
    let s = store(&state);
    let profile = s.profile()?;
    let drives = s.drives()?;
    let rounds = engine::round_labels(&drives);
    drives
        .into_iter()
        .map(|d| {
            let verdict = engine::verdict_in_drive(
                &s,
                &d,
                profile.neo_id.as_deref(),
                profile.reg_no.as_deref(),
            )?;
            // Its place among linked rounds, or the stage the file named:
            // two lists both called "Infosys" are told apart by "Interview"
            // and "Batch 2".
            let round = rounds.get(&d.id).cloned().or(d.round_label.clone());
            Ok(DriveListItem {
                drive: d,
                verdict,
                round,
            })
        })
        .collect()
}

/// A drive as the list shows it: with your answer on it, so the history can
/// be read without opening each one, and its round or stage when known.
#[derive(Debug, Clone, Serialize)]
pub struct DriveListItem {
    #[serde(flatten)]
    pub drive: DriveRecord,
    pub verdict: Verdict,
    pub round: Option<String>,
}

/// Deletes a drive and hands back everything needed to put it back.
///
/// Returning the snapshot is what lets the interface offer Undo instead of a
/// confirmation dialog: the common case costs nothing, and the rare mistake is
/// still recoverable.
#[tauri::command]
pub fn delete_drive(state: tauri::State<AppState>, id: i64) -> R<Option<DriveSnapshot>> {
    let s = store(&state);
    let snap = s.snapshot_drive(id)?;
    s.delete_drive(id)?;
    state.widget_changed();
    Ok(snap)
}

#[tauri::command]
pub fn restore_drive(state: tauri::State<AppState>, snapshot: DriveSnapshot) -> R<i64> {
    let id = store(&state).restore_drive(&snapshot)?;
    state.widget_changed();
    Ok(id)
}

/// Corrects the company name inferred from the filename.
#[tauri::command]
pub fn rename_drive(state: tauri::State<AppState>, id: i64, company: String) -> R<()> {
    let name = company.trim();
    if name.is_empty() {
        return Err(CommandError::new("empty_name", "A drive needs a name."));
    }
    store(&state).rename_drive(id, name)?;
    state.widget_changed();
    Ok(())
}

/// Links a drive as a later round of another, so the comparison can be offered
/// where it is relevant rather than through manual selection.
#[tauri::command]
pub fn set_drive_round(state: tauri::State<AppState>, id: i64, parent_id: Option<i64>) -> R<()> {
    if parent_id == Some(id) {
        return Err(CommandError::new(
            "self_reference",
            "A drive can't be a later round of itself.",
        ));
    }
    store(&state).set_drive_parent(id, parent_id)?;
    state.widget_changed();
    Ok(())
}

#[tauri::command]
pub fn get_drive_detail(state: tauri::State<AppState>, id: i64) -> R<ImportOutcome> {
    let s = store(&state);
    let profile = s.profile()?;
    let identity = engine::build_graph(&s)?;
    drive_outcome(&s, id, &identity, &profile)
}

/// The evidence behind a drive, finding the file again if it has to.
///
/// Drives imported before positions were kept can't show the lines around
/// yours. Their file is usually still in Downloads under the name it was
/// imported as: if one there holds exactly the drive's list, its positions
/// are recorded, as they would have been at import. Nothing else is read, and
/// a file with any other list is left alone. Asked for only when the student
/// opens the evidence, so a permission prompt for Downloads follows a click.
#[tauri::command]
pub fn find_evidence(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    drive_id: i64,
) -> R<engine::Evidence> {
    use tauri::Manager;
    let (drive, profile, evidence) = {
        let s = store(&state);
        let drive = s
            .drive(drive_id)?
            .ok_or_else(|| CommandError::new("not_found", "That drive is no longer stored."))?;
        let profile = s.profile()?;
        let evidence = engine::evidence(&s, &drive, &profile)?;
        (drive, profile, evidence)
    };
    let yours_listed = match (&evidence.key, &evidence.yours) {
        (Some(kind), Some(v)) => store(&state).drive_has(drive_id, *kind, v)?,
        _ => false,
    };
    if !evidence.excerpt.is_empty() || !yours_listed {
        return Ok(evidence);
    }
    let Ok(downloads) = app.path().download_dir() else {
        return Ok(evidence);
    };
    // Read without the store held: a large sheet takes a moment to parse.
    let Some(parsed) = original_file(&downloads, &drive) else {
        return Ok(evidence);
    };
    let s = store(&state);
    s.record_origins(drive_id, &member_origins(&parsed))?;
    Ok(engine::evidence(&s, &drive, &profile)?)
}

/// A drive's own file, if it is still in `downloads` under the name it was
/// imported as (or with a browser's `%20` undone) and still holds exactly the
/// drive's list.
fn original_file(downloads: &Path, drive: &crate::store::DriveRecord) -> Option<parse::ParsedFile> {
    let name = drive.source_filename.as_str();
    let mut names = vec![name.to_string()];
    if name.contains("%20") {
        names.push(name.replace("%20", " "));
    }
    names
        .into_iter()
        .filter(|n| !n.contains(['/', '\\']))
        .map(|n| downloads.join(n))
        .filter(|p| p.is_file())
        .filter_map(|p| parse::parse_file(&p).ok())
        .find(|p| p.content_hash == drive.content_hash)
}

/// Everything the drive screen shows for one drive.
///
/// Shared by the drive screen and the desktop widget, so the widget can never
/// disagree with the app about a verdict, a count or a cutoff. The identity
/// graph is passed in because it is the expensive part, and the widget builds
/// several outcomes from one graph.
pub fn drive_outcome(
    s: &Store,
    id: i64,
    identity: &ResolvedIdentity,
    profile: &Profile,
) -> R<ImportOutcome> {
    let drive = s
        .drive(id)?
        .ok_or_else(|| CommandError::new("not_found", "That drive is no longer stored."))?;

    let neo_ids = s.drive_neo_ids(id)?;
    let reg_nos = s.drive_reg_nos(id)?;
    let primary_key = engine::stored_key(drive.primary_key.as_deref());
    let shape = engine::stored_shape(&drive.shape);

    let synthetic = parse::ParsedFile {
        rows: vec![],
        shape,
        neo_ids,
        reg_nos,
        primary_key,
        content_hash: drive.content_hash.clone(),
        observed_headers: vec![],
        sheet_names: vec![],
        origins: vec![],
        titles: vec![],
    };

    let you = resolve_person(
        "You",
        profile.neo_id.as_deref(),
        profile.reg_no.as_deref(),
        &synthetic,
        identity,
    )?;
    let mut friends = Vec::new();
    for f in s.friends()? {
        friends.push(resolve_person(
            &f.label,
            f.neo_id.as_deref(),
            f.reg_no.as_deref(),
            &synthetic,
            identity,
        )?);
    }
    friends.sort_by_key(|f| match f.verdict {
        Verdict::Shortlisted => 0,
        Verdict::Undetermined(_) => 1,
        Verdict::NotShortlisted => 2,
    });

    let analysis = engine::analyse_drive(s, id, identity, profile)?;
    let evidence = engine::evidence(s, &drive, profile)?;
    let progression = engine::progression(s, id, profile)?;

    Ok(ImportOutcome {
        drive_id: id,
        company: drive.company,
        drive_date: drive.drive_date,
        total_students: drive.total_students,
        shape,
        primary_key,
        you,
        your_cgpa: profile.cgpa.and_then(sanitise_cgpa),
        evidence,
        progression,
        friends,
        analysis,
        learned_verified: 0,
        learned_named: 0,
        unreadable_headers: None,
    })
}

#[tauri::command]
pub fn lookup_identifier(state: tauri::State<AppState>, query: String) -> R<Vec<PersonResult>> {
    let s = store(&state);
    let identity = engine::build_graph(&s)?;
    let q = query.trim();

    let (neo, reg) = (NeoId::parse(q), RegNo::parse(q));
    if neo.is_none() && reg.is_none() {
        return Err(CommandError::new(
            "bad_identifier",
            "Enter a Neo ID (like V9H0G6C4) or a registration number (like 23BAI0002).",
        ));
    }
    let neo_s = neo.as_ref().map(|x| x.as_str().to_string());
    let reg_s = reg.as_ref().map(|x| x.as_str().to_string());
    let resolved = identity.resolve(neo_s.as_deref(), reg_s.as_deref());
    let label = resolved.display_label();

    let mut out = Vec::new();
    for d in s.drives()? {
        let found = match (&neo_s, &reg_s) {
            (Some(n), _) if d.primary_key.as_deref() == Some("neo_id") => {
                s.drive_contains_neo(d.id, n)?
            }
            (_, Some(r)) if d.primary_key.as_deref() == Some("reg_no") => {
                s.drive_contains_reg(d.id, r)?
            }
            _ => continue, // This drive is keyed by something we can't check.
        };
        if found {
            out.push(PersonResult {
                label: format!("{label} — {}", d.company),
                neo_id: neo_s.clone(),
                reg_no: reg_s.clone(),
                verdict: Verdict::Shortlisted,
                confidence: resolved.confidence.label().to_string(),
            });
        }
    }
    Ok(out)
}

#[tauri::command]
pub fn search_students(state: tauri::State<AppState>, query: String) -> R<SearchResult> {
    let s = store(&state);
    let identity = engine::build_graph(&s)?;
    let idx = engine::search_index(&s, &identity)?;

    Ok(match engine::search_by_name(&idx, query.trim()) {
        MatchOutcome::Matched {
            candidate,
            confidence,
        } => SearchResult::Found {
            neo_id: candidate.payload,
            name: candidate.display,
            confidence: confidence.label().to_string(),
        },
        MatchOutcome::Ambiguous { candidates } => SearchResult::Ambiguous {
            candidates: candidates
                .into_iter()
                .map(|c| SearchCandidate {
                    neo_id: c.payload,
                    name: c.display,
                })
                .collect(),
        },
        MatchOutcome::NoMatch => SearchResult::NotFound,
    })
}

/// Round-to-round comparison. Pure set algebra on identifiers — accurate
/// regardless of how little of the roster we can name.
#[tauri::command]
pub fn compare_rounds(state: tauri::State<AppState>, from_id: i64, to_id: i64) -> R<RoundDiff> {
    let s = store(&state);
    let from = s
        .drive(from_id)?
        .ok_or_else(|| CommandError::new("not_found", "The earlier round is no longer stored."))?;
    let to = s
        .drive(to_id)?
        .ok_or_else(|| CommandError::new("not_found", "The later round is no longer stored."))?;

    let a = s.drive_neo_ids(from_id)?;
    let b = s.drive_neo_ids(to_id)?;

    let advanced: Vec<_> = a.intersection(&b).cloned().collect();
    let dropped: Vec<_> = a.difference(&b).cloned().collect();
    let added: Vec<_> = b.difference(&a).cloned().collect();

    let friends = s.friends()?;
    let friend_in = |set: &[NeoId], f: &Friend| -> bool {
        f.neo_id
            .as_deref()
            .and_then(NeoId::parse)
            .map(|n| set.contains(&n))
            .unwrap_or(false)
    };

    Ok(RoundDiff {
        from_company: from.company,
        to_company: to.company,
        advanced: advanced.len(),
        dropped: dropped.len(),
        added: added.len(),
        advanced_friends: friends
            .iter()
            .filter(|f| friend_in(&advanced, f))
            .map(|f| f.label.clone())
            .collect(),
        dropped_friends: friends
            .iter()
            .filter(|f| friend_in(&dropped, f))
            .map(|f| f.label.clone())
            .collect(),
    })
}

// ---------------------------------------------------------------------------
// Reference data, privacy, stats
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ReferenceImportResult {
    pub students_learned: usize,
    pub academics_learned: usize,
    pub verified_links: usize,
    pub kind: String,
}

/// Imports a reference sheet. Minimisation happens during parsing: only
/// identifiers, names, CGPA and branch are read, so contact details cannot
/// reach storage even if the sheet contains them.
#[tauri::command]
pub fn import_reference(state: tauri::State<AppState>, path: String) -> R<ReferenceImportResult> {
    let result = import_reference_inner(state.clone(), path);
    if result.is_ok() {
        // New academic data changes every drive's analysis, not just one.
        state.widget_changed();
    }
    result
}

fn import_reference_inner(state: tauri::State<AppState>, path: String) -> R<ReferenceImportResult> {
    let p = PathBuf::from(&path);
    let filename = p
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "reference.xlsx".into());

    let academic_rows = crate::parse::academic::parse_academic_sheet(&p)
        .map_err(|e| CommandError::new("parse_failed", format!("Couldn't read that file — {e}")))?;

    let s = store(&state);

    if !academic_rows.is_empty() {
        let n = engine::ingest_academics(&s, &academic_rows, &filename)?;
        return Ok(ReferenceImportResult {
            students_learned: n,
            academics_learned: n,
            verified_links: 0,
            kind: "academic".into(),
        });
    }

    // Otherwise treat it as a roster and harvest identity links.
    let parsed = parse::parse_file(&p)
        .map_err(|e| CommandError::new("parse_failed", format!("Couldn't read that file — {e}")))?;
    if !parsed.shape.is_usable() {
        return Err(CommandError::new(
            "file_not_understood",
            "No usable Neo IDs or registration numbers in that file.",
        ));
    }
    let h = engine::harvest_identity(&s, &parsed, &filename)?;
    Ok(ReferenceImportResult {
        students_learned: parsed.neo_ids.len().max(parsed.reg_nos.len()),
        academics_learned: 0,
        verified_links: h.verified_links,
        kind: "roster".into(),
    })
}

#[tauri::command]
pub fn identity_stats(state: tauri::State<AppState>) -> R<IdentityStats> {
    let s = store(&state);
    let identity = engine::build_graph(&s)?;
    Ok(IdentityStats {
        students_known: identity.component_count,
        names_known: identity.neo_to_name.len(),
        academics_known: s.academic_count()?,
        edges: s.edge_count()?,
        conflicts: identity.conflicted_count,
    })
}

#[tauri::command]
pub fn data_inventory(state: tauri::State<AppState>) -> R<Vec<(String, usize)>> {
    Ok(store(&state).inventory()?)
}

/// Deletes everything the student has imported.
///
/// The bundled reference data is restored afterwards, because it is something
/// the application ships rather than something they gave it — leaving analysis
/// permanently broken with no way back would be a strange thing for a "clear my
/// data" control to do. The wording in Settings says so plainly.
#[tauri::command]
pub fn wipe_all_data(state: tauri::State<AppState>) -> R<()> {
    let s = store(&state);
    s.wipe()?;
    if let Err(e) = crate::reference::seed(&s) {
        eprintln!("could not restore bundled reference data: {e}");
    }
    state.widget_changed();
    Ok(())
}

/// The summary for the group chat; see `summary`. Aggregates only, and never
/// the student's own status — that is theirs to disclose.
#[tauri::command]
pub fn share_summary(state: tauri::State<AppState>, drive_id: i64) -> R<String> {
    let s = store(&state);
    let drive = s
        .drive(drive_id)?
        .ok_or_else(|| CommandError::new("not_found", "That drive is no longer stored."))?;
    let profile = s.profile()?;
    let identity = engine::build_graph(&s)?;
    let a = engine::analyse_drive(&s, drive_id, &identity, &profile)?;
    Ok(crate::summary::group_chat(&drive, &s.drives()?, &a))
}

/// Returns, and forgets, a `nankiv://` link that arrived before the interface
/// was listening. Called once the interface has loaded, which covers the case
/// that matters most: a click on the widget that launches the app.
#[tauri::command]
pub fn take_pending_route(state: tauri::State<AppState>) -> Option<crate::widget::Route> {
    state.pending_route.lock().ok().and_then(|mut r| r.take())
}

/// Takes the bytes of a file dropped on the window and puts them somewhere the
/// importer can read.
///
/// macOS hands a dropped file to the web view as content, not as a path — the
/// page is not allowed to know where on disk it came from. Rather than teach
/// the parser a second way in, the bytes are written to a scratch file and the
/// ordinary import runs on that, so a dropped shortlist and one opened from the
/// file panel travel exactly the same road.
#[tauri::command]
pub fn stage_dropped_file(app: tauri::AppHandle, request: tauri::ipc::Request<'_>) -> R<String> {
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(CommandError::new(
            "dropped_file",
            "That file could not be read.",
        ));
    };
    if bytes.len() as u64 > crate::parse::MAX_FILE_BYTES {
        return Err(CommandError::new(
            "dropped_file",
            format!("That file is too large ({} bytes).", bytes.len()),
        ));
    }

    // Percent-encoded by the interface: a header cannot carry a space, and the
    // name matters — the company is taken from it.
    let name = request
        .headers()
        .get("x-filename")
        .and_then(|v| v.to_str().ok())
        .map(|raw| safe_filename(&crate::desktop::percent_decode(raw)))
        .unwrap_or_else(|| "shortlist.xlsx".to_string());

    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| CommandError::new("dropped_file", e.to_string()))?
        .join("dropped");
    std::fs::create_dir_all(&dir).map_err(|e| CommandError::new("dropped_file", e.to_string()))?;
    let path = dir.join(&name);
    std::fs::write(&path, bytes).map_err(|e| CommandError::new("dropped_file", e.to_string()))?;
    Ok(path.to_string_lossy().into_owned())
}

/// The largest card the interface draws, with room to spare.
const MAX_CARD_BYTES: usize = 16 * 1024 * 1024;
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// Saves an "I'm in" card where the student chose in the native save panel.
///
/// The webview has no file access of its own, so the core writes it — and
/// only ever a PNG, to a `.png` path: the bytes must carry PNG's signature.
#[tauri::command]
pub fn save_share_card(request: tauri::ipc::Request<'_>) -> R<()> {
    let refuse = || CommandError::new("share_card", "The card couldn't be saved.");
    let tauri::ipc::InvokeBody::Raw(bytes) = request.body() else {
        return Err(refuse());
    };
    if !bytes.starts_with(PNG_SIGNATURE) || bytes.len() > MAX_CARD_BYTES {
        return Err(refuse());
    }
    let path = request
        .headers()
        .get("x-path")
        .and_then(|v| v.to_str().ok())
        .map(crate::desktop::percent_decode)
        .filter(|p| p.to_lowercase().ends_with(".png"))
        .ok_or_else(refuse)?;
    std::fs::write(&path, bytes).map_err(|e| refuse().with_detail(e.to_string()))
}

/// The name as given, with anything that could point somewhere else removed.
/// The name matters: the importer takes the company from it.
pub fn safe_filename(raw: &str) -> String {
    let trimmed = raw.rsplit(['/', '\\']).next().unwrap_or(raw).trim();
    let cleaned: String = trimmed
        .chars()
        .filter(|c| !matches!(c, '\0'..='\u{1f}' | ':'))
        .collect();
    match cleaned.trim_matches('.').trim() {
        "" => "shortlist.xlsx".to_string(),
        name => name.chars().take(200).collect(),
    }
}

/// Taps the trackpad, for the moment a result lands.
///
/// Silent everywhere else, including on a mouse and wherever the person has
/// turned trackpad feedback off — which is the system's decision to make, not
/// this app's.
#[tauri::command]
pub fn tap() {
    #[cfg(target_os = "macos")]
    {
        extern "C" {
            fn nankiv_tap();
        }
        // SAFETY: takes nothing, returns nothing, and asks AppKit for one
        // haptic tap on the main queue.
        unsafe { nankiv_tap() }
    }
}

/// SF Symbols for the interface's icons, drawn by macOS; see `symbols`.
///
/// One call for every icon on screen, so a view that shows ten of them does not
/// cross the bridge ten times. Each answer is `None` where the system cannot
/// supply the symbol, and the interface draws its own in that place.
#[tauri::command]
pub fn symbols(
    requests: Vec<crate::symbols::SymbolRequest>,
) -> Vec<Option<crate::symbols::SymbolImage>> {
    requests.iter().map(crate::symbols::render).collect()
}

/// How the season is going, for the strip in the toolbar.
#[tauri::command]
pub fn season(state: tauri::State<AppState>) -> R<crate::widget::Season> {
    Ok(crate::widget::season_tally(&store(&state))?)
}

/// Collects shortlists the desktop handed us, once.
#[tauri::command]
pub fn take_pending_files(state: tauri::State<AppState>) -> Vec<String> {
    state
        .pending_files
        .lock()
        .map(|mut f| std::mem::take(&mut *f))
        .unwrap_or_default()
}

/// Writes the names on a shortlist to a file the student chose.
///
/// The path comes from the native save panel, so the file lands exactly where
/// they asked and nowhere else; the webview itself holds no filesystem-write
/// permission. See `export.rs` for what goes into the file and why.
#[tauri::command]
pub fn export_shortlist(
    state: tauri::State<AppState>,
    drive_id: i64,
    path: String,
    format: crate::export::ExportFormat,
) -> R<crate::export::ExportSummary> {
    let s = store(&state);
    let identity = engine::build_graph(&s)?;
    crate::export::export_drive(&s, drive_id, &identity, Path::new(&path), format).map_err(|e| {
        match e {
            crate::export::ExportError::NotFound => {
                CommandError::new("not_found", "That shortlist is no longer stored.")
            }
            crate::export::ExportError::Io(io) => CommandError::new(
                "export_failed",
                "Couldn't save there. Check the folder exists and you can write to it.",
            )
            .with_detail(io.to_string()),
            other => CommandError::new(
                "export_failed",
                format!("Couldn't create the file — {other}"),
            ),
        }
    })
}

#[cfg(test)]
mod dropped_file_tests {
    use super::safe_filename;

    #[test]
    fn the_name_is_kept_because_the_company_comes_from_it() {
        assert_eq!(
            safe_filename("Siemens SISW shortlist 2027.xlsx"),
            "Siemens SISW shortlist 2027.xlsx"
        );
    }

    #[test]
    fn nothing_in_the_name_can_point_somewhere_else() {
        assert_eq!(
            safe_filename("../../.ssh/authorized_keys"),
            "authorized_keys"
        );
        assert_eq!(safe_filename("/etc/passwd"), "passwd");
        assert_eq!(
            safe_filename(r"C:\Windows\system32\drivers\etc\hosts"),
            "hosts"
        );
        assert_eq!(safe_filename("list\u{0}.xlsx"), "list.xlsx");
    }

    #[test]
    fn an_unusable_name_still_gets_a_file() {
        assert_eq!(safe_filename(""), "shortlist.xlsx");
        assert_eq!(safe_filename("   "), "shortlist.xlsx");
        assert_eq!(safe_filename(".."), "shortlist.xlsx");
        assert_eq!(safe_filename("/"), "shortlist.xlsx");
    }

    #[test]
    fn the_name_arrives_percent_encoded() {
        let decoded = crate::desktop::percent_decode("Responsive%20shortlist.xlsx");
        assert_eq!(safe_filename(&decoded), "Responsive shortlist.xlsx");
        assert_eq!(
            safe_filename(&crate::desktop::percent_decode(
                "Siemens%20SISW%20%232.xlsx"
            )),
            "Siemens SISW #2.xlsx"
        );
    }

    #[test]
    fn a_very_long_name_is_trimmed() {
        assert_eq!(safe_filename(&"a".repeat(500)).chars().count(), 200);
    }
}

#[cfg(test)]
mod recent_download_tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn only_the_last_hour_and_only_two_are_offered() {
        let now = SystemTime::now();
        let ago = |mins: u64| now - Duration::from_secs(mins * 60);
        let file = |name: &str| PathBuf::from(name);
        let found = vec![
            (ago(50), file("older.xlsx")),
            (ago(5), file("newest.xlsx")),
            (ago(61), file("over an hour.xlsx")),
            (ago(20), file("middle.xlsx")),
            (ago(60 * 24 * 3), file("days ago.xlsx")),
        ];
        let picked: Vec<_> = just_downloaded(found, now)
            .into_iter()
            .map(|(_, p)| p)
            .collect();
        assert_eq!(picked, [file("newest.xlsx"), file("middle.xlsx")]);
    }

    #[test]
    fn nothing_recent_means_nothing_offered() {
        let now = SystemTime::now();
        let old = vec![(
            now - Duration::from_secs(2 * 60 * 60),
            PathBuf::from("a.xlsx"),
        )];
        assert!(just_downloaded(old, now).is_empty());
    }
}

#[cfg(test)]
mod original_file_tests {
    use super::*;
    use crate::store::DriveRecord;

    const LIST: &str = "S.No,Name,Neo ID\n1,Asha,A1B2C3D4\n2,Ravi,E5F6G7H8\n3,Meena,J9K0L1M2\n";

    fn drive_for(name: &str, content: &str) -> DriveRecord {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("probe.csv");
        std::fs::write(&path, content).unwrap();
        let parsed = parse::parse_file(&path).unwrap();
        DriveRecord {
            id: 1,
            company: "Zluri".into(),
            drive_date: None,
            imported_at: "2026-10-01 09:00:00".into(),
            source_filename: name.into(),
            content_hash: parsed.content_hash,
            shape: "neo_id_only".into(),
            primary_key: Some("neo_id".into()),
            total_students: 3,
            round_label: None,
            parent_drive_id: None,
        }
    }

    #[test]
    fn finds_the_same_list_under_the_name_it_was_imported_as() {
        let downloads = tempfile::tempdir().unwrap();
        std::fs::write(downloads.path().join("Zluri shortlist.csv"), LIST).unwrap();
        let found = original_file(downloads.path(), &drive_for("Zluri shortlist.csv", LIST))
            .expect("the file is there");
        let rows: Vec<_> = member_origins(&found)
            .into_iter()
            .map(|o| (o.value, o.row))
            .collect();
        assert_eq!(
            rows,
            vec![
                ("A1B2C3D4".to_string(), 2),
                ("E5F6G7H8".to_string(), 3),
                ("J9K0L1M2".to_string(), 4),
            ]
        );
    }

    #[test]
    fn undoes_a_browsers_percent_twenty() {
        let downloads = tempfile::tempdir().unwrap();
        std::fs::write(downloads.path().join("Zluri shortlist.csv"), LIST).unwrap();
        assert!(
            original_file(downloads.path(), &drive_for("Zluri%20shortlist.csv", LIST)).is_some()
        );
    }

    #[test]
    fn leaves_a_file_with_any_other_list_alone() {
        // Same name, one student different: not this drive's file.
        let downloads = tempfile::tempdir().unwrap();
        let edited = LIST.replace("J9K0L1M2", "N3P4Q5R6");
        std::fs::write(downloads.path().join("Zluri shortlist.csv"), edited).unwrap();
        assert!(original_file(downloads.path(), &drive_for("Zluri shortlist.csv", LIST)).is_none());
    }

    #[test]
    fn finds_nothing_when_the_file_is_gone_or_the_name_leaves_downloads() {
        let downloads = tempfile::tempdir().unwrap();
        assert!(original_file(downloads.path(), &drive_for("Zluri shortlist.csv", LIST)).is_none());
        assert!(
            original_file(downloads.path(), &drive_for("../Zluri shortlist.csv", LIST)).is_none()
        );
    }
}
