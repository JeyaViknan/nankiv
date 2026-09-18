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
    fn new(code: &str, message: impl Into<String>) -> Self {
        CommandError {
            code: code.into(),
            message: message.into(),
            detail: None,
        }
    }
    fn with_detail(mut self, d: impl Into<String>) -> Self {
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
    /// Only ever populated when the profile opted in *and* the link is strong.
    pub cgpa: Option<f64>,
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
                "That doesn't look like a Neo ID. They're eight characters, alternating letter and digit — like V9H0G6C4.",
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
            "Add a Neo ID or a registration number — a name alone can't be checked against a shortlist.",
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
    use crate::widget::{self, Signal};

    let filename = PathBuf::from(&path)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "shortlist.xlsx".into());

    state.signal(Signal::ImportStarted(filename.clone()));
    let result = import_shortlist_inner(state.clone(), path, company_override, replace_existing);

    // Remember a genuine failure so the widget can explain a missing result,
    // and forget it as soon as any import goes through. A duplicate or a
    // reference sheet is not a failure: the file was a reasonable thing to
    // drop, it just did not add a new drive.
    {
        let s = store(&state);
        let now = time::OffsetDateTime::now_utc();
        let _ = match &result {
            Ok(_) => widget::clear_failure(&s),
            Err(e) if matches!(e.code.as_str(), "duplicate_drive" | "imported_reference") => {
                widget::clear_failure(&s)
            }
            Err(_) => widget::record_failure(&s, &filename, now),
        };
    }
    state.signal(Signal::ImportEnded);
    result
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
    let profile = s.profile()?;

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
                format!(
                    "Added academic records for {learned} students. That's a reference sheet rather than a shortlist — it improves the analysis on every drive you've imported."
                ),
            )
            .with_detail(filename));
        }
    }

    // A file we cannot key on tells us nothing. Report it as such rather than
    // recording an empty drive that would read as a rejection.
    if !parsed.shape.is_usable() {
        let headers = parsed.observed_headers.clone();
        return Err(CommandError::new(
            "file_not_understood",
            "This file doesn't use Neo IDs or registration numbers, so nankiv can't match it to students. Nothing was recorded.",
        )
        .with_detail(if headers.is_empty() {
            "No recognisable column was found.".to_string()
        } else {
            format!("Columns found: {}", headers.join(", "))
        }));
    }

    let (company, drive_date) = match &company_override {
        Some(c) if !c.trim().is_empty() => (c.trim().to_string(), None),
        _ => infer_company_and_date(&filename),
    };

    if replace_existing.unwrap_or(false) {
        if let Some(existing) = s.find_by_hash(&parsed.content_hash)? {
            s.delete_drive(existing.id)?;
        }
    }

    let drive_id = s.insert_drive(
        &company,
        drive_date.as_deref(),
        &filename,
        &parsed.content_hash,
        shape_label(parsed.shape),
        parsed.primary_key.map(key_label),
        &parsed.neo_ids,
        &parsed.reg_nos,
        None,
    )?;

    // Harvest identity links before resolving, so this file improves its own
    // analysis as well as every future one.
    let harvest = engine::harvest_identity(&s, &parsed, &filename)?;
    let identity = engine::build_graph(&s)?;

    let you = resolve_person(
        "You",
        profile.neo_id.as_deref(),
        profile.reg_no.as_deref(),
        &parsed,
        &identity,
        &s,
        profile.show_friend_cgpa,
        true,
    )?;

    let mut friends = Vec::new();
    for f in s.friends()? {
        friends.push(resolve_person(
            &f.label,
            f.neo_id.as_deref(),
            f.reg_no.as_deref(),
            &parsed,
            &identity,
            &s,
            profile.show_friend_cgpa,
            false,
        )?);
    }
    // Shortlisted first, then undetermined, then not shortlisted.
    friends.sort_by_key(|f| match f.verdict {
        Verdict::Shortlisted => 0,
        Verdict::Undetermined(_) => 1,
        Verdict::NotShortlisted => 2,
    });

    let analysis = engine::analyse_drive(&s, drive_id, &identity, &profile)?;

    Ok(ImportOutcome {
        drive_id,
        company,
        drive_date,
        total_students: parsed.student_count(),
        shape: parsed.shape,
        primary_key: parsed.primary_key,
        you,
        friends,
        analysis,
        learned_verified: harvest.verified_links,
        learned_named: harvest.named_links,
        unreadable_headers: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn resolve_person(
    label: &str,
    neo: Option<&str>,
    reg: Option<&str>,
    parsed: &parse::ParsedFile,
    identity: &ResolvedIdentity,
    s: &Store,
    show_cgpa: bool,
    is_self: bool,
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

    // CGPA is shown for the student themselves always, and for friends only
    // when they have explicitly opted in.
    let cgpa = if is_self || show_cgpa {
        resolved
            .reg_no
            .as_ref()
            .and_then(|r| s.academic(r.as_str()).ok().flatten())
            .and_then(|(c, _)| c)
            .and_then(sanitise_cgpa)
    } else {
        None
    };

    Ok(PersonResult {
        label: label.to_string(),
        neo_id: neo.map(|s| s.to_string()),
        reg_no: reg.map(|s| s.to_string()),
        verdict,
        confidence: resolved.confidence.label().to_string(),
        cgpa,
    })
}

/// Infers a company name and date from the filename. Always a suggestion the
/// student can edit, never a silent decision.
pub fn infer_company_and_date(filename: &str) -> (String, Option<String>) {
    let stem = filename
        .rsplit_once('.')
        .map(|(a, _)| a)
        .unwrap_or(filename);

    // Pull a trailing date like 28_07_26 or 1-9-26 before stripping noise.
    let date = once_cell::sync::Lazy::new(|| {
        regex::Regex::new(r"(\d{1,2})[-_](\d{1,2})[-_](\d{2,4})").unwrap()
    });
    let found_date = date
        .captures(stem)
        .map(|c| format!("{}-{}-{}", &c[1], &c[2], &c[3]));

    let mut name = stem.to_string();
    for pat in [
        "shortlisted list",
        "shortlist",
        "shortlisted",
        "test",
        "interview",
        "additonal",
        "additional",
        "with neo id",
        "list",
    ] {
        let re = regex::RegexBuilder::new(&regex::escape(pat))
            .case_insensitive(true)
            .build()
            .expect("static pattern");
        name = re.replace_all(&name, " ").to_string();
    }
    // Strip dates, bracketed suffixes and separators.
    name = regex::Regex::new(r"\d{1,2}[-_]\d{1,2}[-_]\d{2,4}")
        .unwrap()
        .replace_all(&name, " ")
        .to_string();
    name = regex::Regex::new(r"\((\d+)\)")
        .unwrap()
        .replace_all(&name, " ")
        .to_string();
    name = regex::Regex::new(r"[_\-]+")
        .unwrap()
        .replace_all(&name, " ")
        .to_string();
    name = regex::Regex::new(r"\s+")
        .unwrap()
        .replace_all(&name, " ")
        .trim()
        .to_string();

    if name.is_empty() {
        name = "Unnamed drive".to_string();
    }
    (name, found_date)
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
pub fn list_drives(state: tauri::State<AppState>) -> R<Vec<DriveRecord>> {
    Ok(store(&state).drives()?)
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
        return Err(CommandError::new(
            "empty_name",
            "A drive needs a name — otherwise it can't be told apart in your history.",
        ));
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
    let primary_key = match drive.primary_key.as_deref() {
        Some("reg_no") => Some(KeyKind::RegNo),
        Some("neo_id") => Some(KeyKind::NeoId),
        _ => None,
    };
    let shape = match drive.shape.as_str() {
        "reg_no_only" => FileShape::RegNoOnly,
        "linked" => FileShape::Linked,
        "unrecognised" => FileShape::Unrecognised,
        _ => FileShape::NeoIdOnly,
    };

    let synthetic = parse::ParsedFile {
        rows: vec![],
        shape,
        neo_ids,
        reg_nos,
        primary_key,
        content_hash: drive.content_hash.clone(),
        observed_headers: vec![],
        sheet_names: vec![],
    };

    let you = resolve_person(
        "You",
        profile.neo_id.as_deref(),
        profile.reg_no.as_deref(),
        &synthetic,
        identity,
        s,
        profile.show_friend_cgpa,
        true,
    )?;
    let mut friends = Vec::new();
    for f in s.friends()? {
        friends.push(resolve_person(
            &f.label,
            f.neo_id.as_deref(),
            f.reg_no.as_deref(),
            &synthetic,
            identity,
            s,
            profile.show_friend_cgpa,
            false,
        )?);
    }
    friends.sort_by_key(|f| match f.verdict {
        Verdict::Shortlisted => 0,
        Verdict::Undetermined(_) => 1,
        Verdict::NotShortlisted => 2,
    });

    let analysis = engine::analyse_drive(s, id, identity, profile)?;

    Ok(ImportOutcome {
        drive_id: id,
        company: drive.company,
        drive_date: drive.drive_date,
        total_students: drive.total_students,
        shape,
        primary_key,
        you,
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
                cgpa: None,
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
            "That file doesn't contain Neo IDs or registration numbers nankiv can use.",
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

/// A shareable text summary. Aggregates only, and never the student's own
/// status — that is theirs to disclose.
#[tauri::command]
pub fn share_summary(state: tauri::State<AppState>, drive_id: i64) -> R<String> {
    let s = store(&state);
    let drive = s
        .drive(drive_id)?
        .ok_or_else(|| CommandError::new("not_found", "That drive is no longer stored."))?;
    let profile = s.profile()?;
    let identity = engine::build_graph(&s)?;
    let a = engine::analyse_drive(&s, drive_id, &identity, &profile)?;

    let mut out = format!("{} — {} shortlisted", drive.company, drive.total_students);
    if let Some(d) = &drive.drive_date {
        out.push_str(&format!(" ({d})"));
    }
    out.push('\n');

    if a.sufficient {
        if let Some(c) = &a.cutoff {
            out.push_str(&format!("{}\n", c.statement));
        }
        if let Some(b) = &a.branches {
            out.push_str(&format!("{}\n", b.statement));
        }
    } else {
        out.push_str(&format!(
            "Not enough matched students ({} of {}) to analyse CGPA.\n",
            a.matched_students, a.total_students
        ));
    }
    out.push_str("\nvia nankiv");
    Ok(out)
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
    crate::export::export_drive(&s, drive_id, &identity, Path::new(&path), format).map_err(
        |e| match e {
            crate::export::ExportError::NotFound => {
                CommandError::new("not_found", "That shortlist is no longer stored.")
            }
            crate::export::ExportError::Io(io) => CommandError::new(
                "export_failed",
                "Couldn't save the file there. Check that the folder still exists and that you can write to it.",
            )
            .with_detail(io.to_string()),
            other => CommandError::new("export_failed", format!("Couldn't create the file — {other}")),
        },
    )
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
mod tests {
    use super::*;

    #[test]
    fn company_inference_handles_the_real_filenames() {
        let cases = [
            ("Siemens SISW shortlist 2027.xlsx", "Siemens SISW 2027"),
            ("Fractal Analytics shortlist.xlsx", "Fractal Analytics"),
            ("hpe shortlisted list.xlsx", "hpe"),
            ("Tekion additonal shortlist.xlsx", "Tekion"),
            ("amazon shortlist (1).xlsx", "amazon"),
            ("Elgi test shortlist.xlsx", "Elgi"),
            ("Epsilon interview shortlist with neo id.xlsx", "Epsilon"),
            ("BlackRock - Test Shortlist.xlsx", "BlackRock"),
        ];
        for (file, expected) in cases {
            let (got, _) = infer_company_and_date(file);
            assert_eq!(got, expected, "for {file}");
        }
    }

    #[test]
    fn dates_are_pulled_out_of_filenames() {
        let (company, date) = infer_company_and_date("Zluri Shortlist 28_07_26.xlsx");
        assert_eq!(company, "Zluri");
        assert_eq!(date.as_deref(), Some("28-07-26"));

        let (company, date) = infer_company_and_date("Tredence shortlisted list_1-9-26.xlsx");
        assert_eq!(company, "Tredence");
        assert_eq!(date.as_deref(), Some("1-9-26"));
    }

    #[test]
    fn an_unnameable_file_still_gets_a_label() {
        let (company, _) = infer_company_and_date("shortlist.xlsx");
        assert_eq!(company, "Unnamed drive");
    }
}
