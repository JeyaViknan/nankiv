//! The engine: ties parsing, identity and analytics to the store.
//!
//! This is where the reliability hierarchy is actually enforced at runtime.
//! Every path that could answer "not shortlisted" goes through
//! [`membership_verdict`], which refuses to do so unless the file is keyed by
//! something the caller actually holds.

use crate::analytics::{Baseline, DriveAnalysis};
use crate::identity::{
    graph::IdentityGraph,
    matcher::{CorpusIndex, MatchOutcome},
    name,
};
use crate::model::*;
use crate::parse::{FileShape, ParsedFile};
use crate::store::{DriveRecord, Profile, Store, StoreError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Resolves a membership question without ever guessing.
///
/// The single rule: we may only answer `NotShortlisted` when the file is keyed
/// by an identifier kind the subject actually has. Otherwise their absence from
/// the file is evidence of nothing.
pub fn membership_verdict(
    file_key: Option<KeyKind>,
    shape: FileShape,
    subject_neo: Option<&str>,
    subject_reg: Option<&str>,
    neo_ids: &BTreeSet<NeoId>,
    reg_nos: &BTreeSet<RegNo>,
) -> Verdict {
    membership_verdict_by(
        file_key,
        shape,
        subject_neo,
        subject_reg,
        |n| neo_ids.contains(n),
        |r| reg_nos.contains(r),
    )
}

/// [`membership_verdict`], with the lookups supplied by the caller, so a
/// stored drive can be answered without loading every member it lists. The
/// rule is the same and lives in one place.
pub fn membership_verdict_by(
    file_key: Option<KeyKind>,
    shape: FileShape,
    subject_neo: Option<&str>,
    subject_reg: Option<&str>,
    has_neo: impl FnOnce(&NeoId) -> bool,
    has_reg: impl FnOnce(&RegNo) -> bool,
) -> Verdict {
    if !shape.is_usable() {
        return Verdict::Undetermined(Undetermined::FileNotUnderstood);
    }
    let Some(key) = file_key else {
        return Verdict::Undetermined(Undetermined::FileNotUnderstood);
    };
    if subject_neo.is_none() && subject_reg.is_none() {
        return Verdict::Undetermined(Undetermined::NoIdentityConfigured);
    }

    match key {
        KeyKind::NeoId => match subject_neo.and_then(NeoId::parse) {
            Some(id) => Verdict::from_lookup(has_neo(&id)),
            // The file is keyed by Neo ID and we don't have one for this person.
            // Their absence means nothing.
            None => Verdict::Undetermined(Undetermined::KeyKindNotConfigured {
                file_key: KeyKind::NeoId,
            }),
        },
        KeyKind::RegNo => match subject_reg.and_then(RegNo::parse) {
            Some(id) => Verdict::from_lookup(has_reg(&id)),
            None => Verdict::Undetermined(Undetermined::KeyKindNotConfigured {
                file_key: KeyKind::RegNo,
            }),
        },
    }
}

/// The key a stored drive was recorded with.
pub fn stored_key(label: Option<&str>) -> Option<KeyKind> {
    match label {
        Some("reg_no") => Some(KeyKind::RegNo),
        Some("neo_id") => Some(KeyKind::NeoId),
        _ => None,
    }
}

/// The shape a stored drive was recorded with.
pub fn stored_shape(label: &str) -> FileShape {
    match label {
        "reg_no_only" => FileShape::RegNoOnly,
        "linked" => FileShape::Linked,
        "unrecognised" => FileShape::Unrecognised,
        _ => FileShape::NeoIdOnly,
    }
}

/// A person's verdict on a stored drive, asked of the store directly.
pub fn verdict_in_drive(
    store: &Store,
    drive: &DriveRecord,
    neo: Option<&str>,
    reg: Option<&str>,
) -> Result<Verdict, StoreError> {
    // Asked up front so a storage error is an error, not a "no".
    let neo_in = match neo.and_then(NeoId::parse) {
        Some(n) => store.drive_has(drive.id, KeyKind::NeoId, n.as_str())?,
        None => false,
    };
    let reg_in = match reg.and_then(RegNo::parse) {
        Some(r) => store.drive_has(drive.id, KeyKind::RegNo, r.as_str())?,
        None => false,
    };
    Ok(membership_verdict_by(
        stored_key(drive.primary_key.as_deref()),
        stored_shape(&drive.shape),
        neo,
        reg,
        |_| neo_in,
        |_| reg_in,
    ))
}

// ---------------------------------------------------------------------------
// Rounds
// ---------------------------------------------------------------------------

/// How much of a new list must have been on an earlier one, from the same
/// company, for the two to be read as rounds of one drive. A later round is
/// drawn from the earlier one; two roles at the same company barely overlap.
/// Below this, they are left apart: a wrong link is worse than none.
pub const ROUND_OVERLAP: f64 = 0.6;

fn company_key(company: &str) -> String {
    company
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn share_of<T: Ord>(new: &BTreeSet<T>, earlier: &BTreeSet<T>) -> Option<f64> {
    (!new.is_empty() && !earlier.is_empty())
        .then(|| new.intersection(earlier).count() as f64 / new.len() as f64)
}

/// Links a newly stored drive to the earlier round it continues, if one does.
pub fn link_round(store: &Store, drive_id: i64) -> Result<Option<i64>, StoreError> {
    let Some(new) = store.drive(drive_id)? else {
        return Ok(None);
    };
    let key = company_key(&new.company);
    if new.parent_drive_id.is_some() || key == company_key(crate::naming::UNNAMED) {
        return Ok(new.parent_drive_id);
    }
    let (new_neo, new_reg) = (
        store.drive_neo_ids(drive_id)?,
        store.drive_reg_nos(drive_id)?,
    );

    // Newest first, so a third round attaches to the second, not the first.
    for earlier in store.drives()? {
        let before =
            (earlier.imported_at.as_str(), earlier.id) < (new.imported_at.as_str(), new.id);
        if !before || company_key(&earlier.company) != key {
            continue;
        }
        let share = share_of(&new_neo, &store.drive_neo_ids(earlier.id)?)
            .or(share_of(&new_reg, &store.drive_reg_nos(earlier.id)?));
        if share.is_some_and(|s| s >= ROUND_OVERLAP) {
            store.set_drive_parent(drive_id, Some(earlier.id))?;
            return Ok(Some(earlier.id));
        }
    }
    Ok(None)
}

/// Gives drives imported under the old naming rules the names today's rules
/// would, once. Only a name the old rules produced is replaced: if it differs
/// from what they make of the drive's filename, a student chose it, and it is
/// left alone. Every drive also gets the stage its filename names, if any.
pub fn upgrade_legacy_names(store: &Store) -> Result<usize, StoreError> {
    const DONE: &str = "names_upgraded_v1";
    if store.meta(DONE)?.is_some() {
        return Ok(0);
    }
    let mut renamed = 0;
    for d in store.drives()? {
        let fresh = crate::naming::infer(&d.source_filename, &[], &[]);
        if d.company == crate::naming::legacy_name(&d.source_filename) && fresh.company != d.company
        {
            store.rename_drive(d.id, &fresh.company)?;
            renamed += 1;
        }
        if d.round_label.is_none() && fresh.round.is_some() {
            store.set_round_label(d.id, fresh.round.as_deref())?;
        }
    }
    store.set_meta(DONE, "1")?;
    Ok(renamed)
}

/// Links rounds among drives imported before linking existed, oldest first,
/// by the same rule as an import. Runs once per database: a link the student
/// has since separated is never quietly made again.
pub fn link_existing_rounds(store: &Store) -> Result<usize, StoreError> {
    // v2: names were improved after v1 ran, which can reveal rounds it missed.
    const DONE: &str = "rounds_linked_v2";
    if store.meta(DONE)?.is_some() {
        return Ok(0);
    }
    let mut linked = 0;
    for d in store.drives()?.into_iter().rev() {
        if d.parent_drive_id.is_none() && link_round(store, d.id)?.is_some() {
            linked += 1;
        }
    }
    store.set_meta(DONE, "1")?;
    Ok(linked)
}

/// One round in a progression, with your answer in it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoundStep {
    pub drive_id: i64,
    /// What the file called the round, or its position: "R1", "R2", "Final".
    pub label: String,
    pub verdict: Verdict,
}

/// A drive's rounds, earliest first: `R1 ✓ → R2 ✓ → next ?`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Progression {
    pub steps: Vec<RoundStep>,
    /// You are in the latest round, and nothing says it was the last.
    pub next_pending: bool,
}

/// The label of every drive that is part of a chain of rounds: the file's own
/// word for it where it gave one, its position otherwise.
pub fn round_labels(drives: &[DriveRecord]) -> BTreeMap<i64, String> {
    let by_id: BTreeMap<i64, &DriveRecord> = drives.iter().map(|d| (d.id, d)).collect();
    let has_child: BTreeSet<i64> = drives.iter().filter_map(|d| d.parent_drive_id).collect();
    let mut labels = BTreeMap::new();
    for d in drives {
        let mut depth = 0;
        let mut seen = BTreeSet::from([d.id]);
        let mut cur = d.parent_drive_id;
        while let Some(p) = cur.filter(|p| by_id.contains_key(p) && seen.insert(*p)) {
            depth += 1;
            cur = by_id[&p].parent_drive_id;
        }
        if depth > 0 || has_child.contains(&d.id) {
            let label = d.round_label.clone().unwrap_or(format!("R{}", depth + 1));
            labels.insert(d.id, label);
        }
    }
    labels
}

/// The rounds around one drive, and your answer in each.
pub fn progression(
    store: &Store,
    drive_id: i64,
    profile: &Profile,
) -> Result<Option<Progression>, StoreError> {
    let drives = store.drives()?;
    let by_id: BTreeMap<i64, &DriveRecord> = drives.iter().map(|d| (d.id, d)).collect();
    if !by_id.contains_key(&drive_id) {
        return Ok(None);
    }

    // Up to the first round...
    let mut seen = BTreeSet::from([drive_id]);
    let mut path = vec![drive_id];
    let mut cur = by_id[&drive_id].parent_drive_id;
    while let Some(p) = cur.filter(|p| by_id.contains_key(p) && seen.insert(*p)) {
        path.push(p);
        cur = by_id[&p].parent_drive_id;
    }
    path.reverse();
    // ...and down through the latest round that followed this one. `drives`
    // is newest first, so the first child found is the latest.
    let mut cur = drive_id;
    while let Some(next) = drives
        .iter()
        .find(|d| d.parent_drive_id == Some(cur) && !seen.contains(&d.id))
    {
        seen.insert(next.id);
        path.push(next.id);
        cur = next.id;
    }
    if path.len() < 2 {
        return Ok(None);
    }

    let labels = round_labels(&drives);
    let mut steps = Vec::new();
    for id in path {
        let d = by_id[&id];
        steps.push(RoundStep {
            drive_id: id,
            label: labels.get(&id).cloned().unwrap_or_default(),
            verdict: verdict_in_drive(
                store,
                d,
                profile.neo_id.as_deref(),
                profile.reg_no.as_deref(),
            )?,
        });
    }
    let last = steps.last().expect("at least two rounds");
    let next_pending = last.verdict == Verdict::Shortlisted && last.label != "Final";
    Ok(Some(Progression {
        steps,
        next_pending,
    }))
}

// ---------------------------------------------------------------------------
// Evidence
// ---------------------------------------------------------------------------

/// What a verdict rests on, in terms a student can check against the file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Evidence {
    /// What the file lists students by.
    pub key: Option<KeyKind>,
    /// Your identifier of that kind, as you entered it.
    pub yours: Option<String>,
    /// How many identifiers of that kind the file lists.
    pub listed: usize,
    /// Where yours appears, when it does and the file's layout was recorded.
    /// Drives imported before positions were kept have none.
    pub found_at: Option<crate::store::MemberOrigin>,
}

/// The evidence behind your verdict on a stored drive.
pub fn evidence(
    store: &Store,
    drive: &DriveRecord,
    profile: &Profile,
) -> Result<Evidence, StoreError> {
    let key = stored_key(drive.primary_key.as_deref());
    let (kind, yours) = match key {
        Some(KeyKind::NeoId) => (
            "neo_id",
            profile
                .neo_id
                .as_deref()
                .and_then(NeoId::parse)
                .map(|n| n.as_str().to_string()),
        ),
        Some(KeyKind::RegNo) => (
            "reg_no",
            profile
                .reg_no
                .as_deref()
                .and_then(RegNo::parse)
                .map(|r| r.as_str().to_string()),
        ),
        None => ("", None),
    };
    let found_at = match &yours {
        Some(v) => store.origin(drive.id, kind, v)?,
        None => None,
    };
    Ok(Evidence {
        key,
        yours,
        listed: drive.total_students,
        found_at,
    })
}

/// Loads the whole identity graph from storage and resolves it.
pub fn build_graph(store: &Store) -> Result<ResolvedIdentity, StoreError> {
    let mut g = IdentityGraph::new();
    for (a, b, conf, src) in store.edges()? {
        g.link(&a, &b, conf, &src);
    }
    for (key, display) in store.spellings()? {
        g.remember_spelling(&key, &display);
    }

    let comps = g.components();
    let mut neo_to_reg: BTreeMap<String, (String, Confidence)> = BTreeMap::new();
    let mut neo_to_name: BTreeMap<String, (String, Confidence)> = BTreeMap::new();
    let mut reg_to_name: BTreeMap<String, (String, Confidence)> = BTreeMap::new();
    let mut name_to_neo: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for c in &comps {
        if c.conflicted {
            continue; // A contradicted component teaches us nothing reliable.
        }
        let reg = c.reg_nos.iter().next().map(|r| r.as_str().to_string());
        let display_name = c
            .best_name()
            .and_then(|k| g.spelling_of(k).map(|s| s.to_string()));

        for n in &c.neo_ids {
            if let Some(r) = &reg {
                neo_to_reg.insert(n.as_str().to_string(), (r.clone(), c.confidence));
            }
            if let Some(nm) = &display_name {
                neo_to_name.insert(n.as_str().to_string(), (nm.clone(), c.confidence));
            }
        }
        if let (Some(r), Some(nm)) = (&reg, &display_name) {
            reg_to_name.insert(r.clone(), (nm.clone(), c.confidence));
        }
        // Name search index — only where the component may be named.
        if c.confidence.can_name_person() {
            for k in c.names.keys() {
                let entry = name_to_neo.entry(k.clone()).or_default();
                for n in &c.neo_ids {
                    entry.push(n.as_str().to_string());
                }
            }
        }
    }

    Ok(ResolvedIdentity {
        neo_to_reg,
        neo_to_name,
        reg_to_name,
        name_to_neo,
        spellings: g,
        component_count: comps.len(),
        conflicted_count: comps.iter().filter(|c| c.conflicted).count(),
    })
}

pub struct ResolvedIdentity {
    pub neo_to_reg: BTreeMap<String, (String, Confidence)>,
    pub neo_to_name: BTreeMap<String, (String, Confidence)>,
    pub reg_to_name: BTreeMap<String, (String, Confidence)>,
    pub name_to_neo: BTreeMap<String, Vec<String>>,
    pub spellings: IdentityGraph,
    pub component_count: usize,
    pub conflicted_count: usize,
}

impl ResolvedIdentity {
    /// The name for a Neo ID, or `None` when nothing is strong enough to show.
    pub fn name_of_neo(&self, neo: &str) -> Option<&str> {
        self.neo_to_name
            .get(neo)
            .filter(|(_, c)| c.can_name_person())
            .map(|(n, _)| n.as_str())
    }

    /// The name for a registration number, under the same rule as
    /// [`name_of_neo`](Self::name_of_neo): nothing short of a link strong
    /// enough to show a person.
    pub fn name_of_reg(&self, reg: &str) -> Option<&str> {
        self.reg_to_name
            .get(reg)
            .filter(|(_, c)| c.can_name_person())
            .map(|(n, _)| n.as_str())
    }

    /// Registration number for a Neo ID, at any confidence that may aggregate.
    pub fn reg_of_neo(&self, neo: &str) -> Option<&str> {
        self.neo_to_reg
            .get(neo)
            .filter(|(_, c)| c.can_aggregate())
            .map(|(r, _)| r.as_str())
    }

    pub fn confidence_of_neo(&self, neo: &str) -> Confidence {
        self.neo_to_name
            .get(neo)
            .map(|(_, c)| *c)
            .or_else(|| self.neo_to_reg.get(neo).map(|(_, c)| *c))
            .unwrap_or(Confidence::Unresolved)
    }

    pub fn resolve(&self, neo: Option<&str>, reg: Option<&str>) -> ResolvedStudent {
        let name = neo
            .and_then(|n| self.name_of_neo(n))
            .or_else(|| reg.and_then(|r| self.reg_to_name.get(r).map(|(n, _)| n.as_str())))
            .map(|s| s.to_string());
        let confidence = neo
            .map(|n| self.confidence_of_neo(n))
            .unwrap_or(Confidence::Unresolved);
        ResolvedStudent {
            neo_id: neo.and_then(NeoId::parse),
            reg_no: reg
                .and_then(RegNo::parse)
                .or_else(|| neo.and_then(|n| self.reg_of_neo(n)).and_then(RegNo::parse)),
            name,
            confidence,
        }
    }
}

/// Harvests identity links from a parsed file and writes them to the store.
///
/// Rows carrying more than one identifier type become `Verified` edges. This is
/// the mechanism that lets one rich file permanently improve every future
/// import.
pub fn harvest_identity(
    store: &Store,
    parsed: &ParsedFile,
    source: &str,
) -> Result<HarvestReport, StoreError> {
    let mut verified = 0usize;
    let mut named = 0usize;

    for row in &parsed.rows {
        let ids = row.identifiers();
        // Every identifier pair observed in the same row is a fact.
        for i in 0..ids.len() {
            for j in (i + 1)..ids.len() {
                store.add_edge(&ids[i], &ids[j], Confidence::Verified, source)?;
                verified += 1;
            }
        }
        // A name in the same row is strong, but a name is not an identifier: it
        // is recorded at `High`, never `Verified`.
        if let Some(nm) = &row.name {
            let key = name::name_key(nm);
            if key.is_empty() {
                continue;
            }
            store.remember_spelling(&key, nm)?;
            let name_id = Identifier::NameKey(key);
            for id in &ids {
                store.add_edge(id, &name_id, Confidence::High, source)?;
                named += 1;
            }
        }
    }

    Ok(HarvestReport {
        verified_links: verified,
        named_links: named,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarvestReport {
    pub verified_links: usize,
    pub named_links: usize,
}

/// Ingests a reference sheet holding academic data.
///
/// Minimisation happens here: only registration number, CGPA and branch are
/// read. Phone, email, date of birth, gender, resume links and 10th/12th marks
/// are never extracted, so they cannot reach storage.
pub struct AcademicRow {
    pub reg_no: RegNo,
    pub name: Option<String>,
    pub cgpa: Option<f64>,
    pub branch: Option<String>,
}

pub fn ingest_academics(
    store: &Store,
    rows: &[AcademicRow],
    source: &str,
) -> Result<usize, StoreError> {
    let mut n = 0;
    for r in rows {
        store.upsert_academic(&r.reg_no, r.cgpa, r.branch.as_deref(), source)?;
        if let Some(nm) = &r.name {
            let key = name::name_key(nm);
            if !key.is_empty() {
                store.remember_spelling(&key, nm)?;
                store.add_edge(
                    &Identifier::RegNo(r.reg_no.clone()),
                    &Identifier::NameKey(key),
                    Confidence::High,
                    source,
                )?;
            }
        }
        // The baseline is anonymous: values only, no identifier stored beside them.
        store.add_baseline(r.reg_no.admission_year(), r.cgpa, r.branch.as_deref())?;
        n += 1;
    }
    Ok(n)
}

/// Bridges Neo IDs to registration numbers through the names attached to each.
///
/// Names are attributes rather than graph nodes — deliberately, so that three
/// students called `Naveen` never collapse into one. The consequence is that a
/// name alone never joins anything, and until this pass ran the whole
/// name-matching apparatus contributed nothing to identity: coverage came only
/// from files that happened to print both identifiers in the same row.
///
/// So the join is made explicitly, and only where it is safe. Every candidate
/// goes through the ambiguity gate; anything the evidence cannot separate is
/// left unresolved. An exact name is recorded as `High` and may be shown; a
/// phonetic or typo-level match is `Probable`, which counts toward statistics
/// but is never allowed to put a name on screen next to a person.
pub fn link_by_name(store: &Store) -> Result<NameLinkReport, StoreError> {
    // Candidates: every registration number we hold academic data for, indexed
    // by the names seen for it.
    let mut corpus = CorpusIndex::new();
    let spellings = store.spellings()?;
    let by_key: std::collections::HashMap<&str, &str> = spellings
        .iter()
        .map(|(k, d)| (k.as_str(), d.as_str()))
        .collect();

    for (a, b, _, _) in store.edges()? {
        if let (Identifier::RegNo(r), Identifier::NameKey(k))
        | (Identifier::NameKey(k), Identifier::RegNo(r)) = (&a, &b)
        {
            let display = by_key.get(k.as_str()).copied().unwrap_or(k.as_str());
            corpus.insert(display, r.as_str());
        }
    }
    if corpus.is_empty() {
        return Ok(NameLinkReport::default());
    }

    // Queries: every Neo ID that has a name but no registration number yet.
    let identity = build_graph(store)?;
    let mut report = NameLinkReport::default();

    for (neo, (display, conf)) in &identity.neo_to_name {
        if identity.neo_to_reg.contains_key(neo) {
            continue; // already resolved, and by something stronger
        }
        if !conf.can_aggregate() {
            continue;
        }
        let Some(neo_id) = NeoId::parse(neo) else {
            continue;
        };

        match corpus.match_name(display) {
            MatchOutcome::Matched {
                candidate,
                confidence,
            } => {
                let Some(reg) = RegNo::parse(&candidate.payload) else {
                    continue;
                };
                // The link is only as strong as the weaker of the two claims.
                let strength = confidence.min(*conf);
                if !strength.can_aggregate() {
                    continue;
                }
                store.add_edge(
                    &Identifier::NeoId(neo_id),
                    &Identifier::RegNo(reg),
                    strength,
                    "name-match",
                )?;
                if strength == Confidence::High {
                    report.exact += 1;
                } else {
                    report.approximate += 1;
                }
            }
            MatchOutcome::Ambiguous { .. } => report.ambiguous += 1,
            MatchOutcome::NoMatch => report.unmatched += 1,
        }
    }

    Ok(report)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NameLinkReport {
    /// Exact name agreement, unique on both sides.
    pub exact: usize,
    /// Phonetic or typo-level agreement. Counts toward statistics only.
    pub approximate: usize,
    /// More than one candidate. Deliberately left unresolved.
    pub ambiguous: usize,
    pub unmatched: usize,
}

/// Builds a name-search index from everything the graph can safely name.
pub fn search_index(store: &Store, identity: &ResolvedIdentity) -> Result<CorpusIndex, StoreError> {
    let mut idx = CorpusIndex::new();
    for (key, display) in store.spellings()? {
        if let Some(neos) = identity.name_to_neo.get(&key) {
            for n in neos {
                idx.insert(&display, n);
            }
        }
    }
    Ok(idx)
}

/// Resolves a free-text name query to Neo IDs, refusing ambiguity.
pub fn search_by_name(idx: &CorpusIndex, query: &str) -> MatchOutcome {
    idx.match_name(query)
}

/// Runs the analytics for one drive.
pub fn analyse_drive(
    store: &Store,
    drive_id: i64,
    identity: &ResolvedIdentity,
    profile: &Profile,
) -> Result<DriveAnalysis, StoreError> {
    let neo_ids = store.drive_neo_ids(drive_id)?;
    let reg_nos = store.drive_reg_nos(drive_id)?;
    let total = if neo_ids.len() >= reg_nos.len() {
        neo_ids.len()
    } else {
        reg_nos.len()
    };

    // Resolve each shortlisted student to academic data.
    let mut cgpas = Vec::new();
    let mut branches = Vec::new();

    let mut resolved_regs: BTreeSet<String> = BTreeSet::new();
    for n in &neo_ids {
        if let Some(r) = identity.reg_of_neo(n.as_str()) {
            resolved_regs.insert(r.to_string());
        }
    }
    for r in &reg_nos {
        resolved_regs.insert(r.as_str().to_string());
    }

    for r in &resolved_regs {
        if let Some((cgpa, branch)) = store.academic(r)? {
            if let Some(c) = cgpa.and_then(sanitise_cgpa) {
                cgpas.push(c);
                branches.push(branch.unwrap_or_default());
            }
        }
    }

    // Baseline filtered to the student's own cohort where known.
    let (base_cgpa, base_branch) = store.baseline(profile.cohort.as_deref())?;
    let (base_cgpa, base_branch) = if base_cgpa.is_empty() {
        store.baseline(None)?
    } else {
        (base_cgpa, base_branch)
    };
    let canon_branches: Vec<String> = base_branch
        .iter()
        .map(|b| crate::analytics::branch::canonicalise(b))
        .collect();
    let baseline = Baseline::from_values(&base_cgpa).with_branches(&canon_branches);

    // The student's own CGPA, for the standing figure: the one they entered.
    // Never the reference data's figure for their identifier — that identifier
    // is unverified, so it would show anyone's CGPA to whoever typed their ID.
    let your_cgpa = profile.cgpa.and_then(sanitise_cgpa);

    Ok(DriveAnalysis::build(
        &cgpas, &branches, total, &baseline, your_cgpa,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn neos(v: &[&str]) -> BTreeSet<NeoId> {
        v.iter().filter_map(|s| NeoId::parse(s)).collect()
    }
    fn regs(v: &[&str]) -> BTreeSet<RegNo> {
        v.iter().filter_map(|s| RegNo::parse(s)).collect()
    }

    #[test]
    fn present_in_a_neo_keyed_file_is_shortlisted() {
        let v = membership_verdict(
            Some(KeyKind::NeoId),
            FileShape::NeoIdOnly,
            Some("V9H0G6C4"),
            None,
            &neos(&["V9H0G6C4", "C5U6K1E7"]),
            &BTreeSet::new(),
        );
        assert_eq!(v, Verdict::Shortlisted);
    }

    #[test]
    fn absent_from_a_neo_keyed_file_is_not_shortlisted() {
        let v = membership_verdict(
            Some(KeyKind::NeoId),
            FileShape::NeoIdOnly,
            Some("T2D4R9N9"),
            None,
            &neos(&["V9H0G6C4"]),
            &BTreeSet::new(),
        );
        assert_eq!(v, Verdict::NotShortlisted);
    }

    #[test]
    fn a_reg_keyed_file_never_rejects_a_student_who_only_has_a_neo_id() {
        // The critical case. Two of fifteen real files are reg-keyed. A student
        // with only a Neo ID is simply unanswerable here, and saying "not
        // shortlisted" would be a lie.
        let v = membership_verdict(
            Some(KeyKind::RegNo),
            FileShape::RegNoOnly,
            Some("V9H0G6C4"),
            None,
            &BTreeSet::new(),
            &regs(&["23BAI0001", "23BAI0011"]),
        );
        assert_eq!(
            v,
            Verdict::Undetermined(Undetermined::KeyKindNotConfigured {
                file_key: KeyKind::RegNo
            })
        );
        assert_ne!(v, Verdict::NotShortlisted);
    }

    #[test]
    fn an_unreadable_file_is_never_a_rejection() {
        // The TCS/Cognizant case. Reporting "not shortlisted" here would be the
        // worst possible bug in this product.
        let v = membership_verdict(
            None,
            FileShape::Unrecognised,
            Some("V9H0G6C4"),
            Some("23BAI0001"),
            &BTreeSet::new(),
            &BTreeSet::new(),
        );
        assert_eq!(v, Verdict::Undetermined(Undetermined::FileNotUnderstood));
        assert_ne!(v, Verdict::NotShortlisted);
    }

    #[test]
    fn no_identity_configured_is_undetermined_not_a_rejection() {
        let v = membership_verdict(
            Some(KeyKind::NeoId),
            FileShape::NeoIdOnly,
            None,
            None,
            &neos(&["V9H0G6C4"]),
            &BTreeSet::new(),
        );
        assert_eq!(v, Verdict::Undetermined(Undetermined::NoIdentityConfigured));
    }

    #[test]
    fn a_student_with_both_ids_is_answerable_in_either_file_shape() {
        for (key, shape) in [
            (KeyKind::NeoId, FileShape::NeoIdOnly),
            (KeyKind::RegNo, FileShape::RegNoOnly),
        ] {
            let v = membership_verdict(
                Some(key),
                shape,
                Some("V9H0G6C4"),
                Some("23BAI0001"),
                &neos(&["V9H0G6C4"]),
                &regs(&["23BAI0001"]),
            );
            assert_eq!(v, Verdict::Shortlisted, "failed for {key:?}");
        }
    }

    #[test]
    fn harvesting_a_linked_row_creates_verified_edges() {
        let s = Store::open_in_memory().unwrap();
        let parsed = ParsedFile {
            rows: vec![crate::parse::ParsedRow {
                neo_id: NeoId::parse("E2S8L9L8"),
                reg_no: RegNo::parse("23BCE1473"),
                name: Some("Monish D".into()),
            }],
            shape: FileShape::Linked,
            neo_ids: neos(&["E2S8L9L8"]),
            reg_nos: regs(&["23BCE1473"]),
            primary_key: Some(KeyKind::NeoId),
            content_hash: "H".into(),
            observed_headers: vec![],
            sheet_names: vec!["Sheet1".into()],
            origins: vec![],
            titles: vec![],
        };
        let r = harvest_identity(&s, &parsed, "tredence").unwrap();
        assert_eq!(r.verified_links, 1, "neo <-> reg");
        assert_eq!(r.named_links, 2, "name linked to both identifiers");

        let id = build_graph(&s).unwrap();
        assert_eq!(id.reg_of_neo("E2S8L9L8"), Some("23BCE1473"));
        assert_eq!(id.name_of_neo("E2S8L9L8"), Some("Monish D"));
    }

    #[test]
    fn a_probable_link_resolves_but_does_not_name() {
        let s = Store::open_in_memory().unwrap();
        let neo = Identifier::NeoId(NeoId::parse("V9H0G6C4").unwrap());
        let nm = Identifier::NameKey(name::name_key("Maybe Person"));
        s.remember_spelling(&name::name_key("Maybe Person"), "Maybe Person")
            .unwrap();
        s.add_edge(&neo, &nm, Confidence::Probable, "fuzzy")
            .unwrap();

        let id = build_graph(&s).unwrap();
        assert_eq!(
            id.name_of_neo("V9H0G6C4"),
            None,
            "probable must never surface a name"
        );
    }

    #[test]
    fn a_shared_name_does_not_merge_two_students() {
        // Two different students named `Naveen`. Because names are attributes
        // rather than graph nodes, they must remain two components — and both
        // stay nameable.
        let s = Store::open_in_memory().unwrap();
        let k = name::name_key("Naveen");
        s.remember_spelling(&k, "Naveen").unwrap();
        for (neo, reg) in [("V9H0G6C4", "23AAA0001"), ("C5U6K1E7", "23BBB0002")] {
            let n = Identifier::NeoId(NeoId::parse(neo).unwrap());
            s.add_edge(
                &n,
                &Identifier::RegNo(RegNo::parse(reg).unwrap()),
                Confidence::Verified,
                "f",
            )
            .unwrap();
            s.add_edge(&n, &Identifier::NameKey(k.clone()), Confidence::High, "f")
                .unwrap();
        }
        let id = build_graph(&s).unwrap();
        assert_eq!(id.component_count, 2, "must not collapse into one student");
        assert_eq!(id.conflicted_count, 0);
        assert_eq!(id.reg_of_neo("V9H0G6C4"), Some("23AAA0001"));
        assert_eq!(id.reg_of_neo("C5U6K1E7"), Some("23BBB0002"));
    }

    #[test]
    fn contradictory_identifier_links_are_dropped_entirely() {
        // One Neo ID claimed against two registration numbers is a bad merge.
        let s = Store::open_in_memory().unwrap();
        let n = Identifier::NeoId(NeoId::parse("V9H0G6C4").unwrap());
        let k = name::name_key("Kumar Gupta");
        s.remember_spelling(&k, "Kumar Gupta").unwrap();
        s.add_edge(&n, &Identifier::NameKey(k), Confidence::High, "g")
            .unwrap();
        for reg in ["23AAA0001", "23BBB0002"] {
            s.add_edge(
                &n,
                &Identifier::RegNo(RegNo::parse(reg).unwrap()),
                Confidence::Probable,
                "guess",
            )
            .unwrap();
        }
        let id = build_graph(&s).unwrap();
        assert_eq!(id.conflicted_count, 1);
        assert!(id.reg_to_name.is_empty(), "nothing usable from a conflict");
        assert_eq!(
            id.name_of_neo("V9H0G6C4"),
            None,
            "a contradicted student must never be named"
        );
    }

    #[test]
    fn academics_ingestion_records_only_minimal_fields() {
        let s = Store::open_in_memory().unwrap();
        let rows = vec![AcademicRow {
            reg_no: RegNo::parse("23BAI0002").unwrap(),
            name: Some("Yogdeep Benchimath".into()),
            cgpa: Some(8.24),
            branch: Some("CSE (AIML)".into()),
        }];
        assert_eq!(ingest_academics(&s, &rows, "sheet").unwrap(), 1);
        let a = s.academic("23BAI0002").unwrap().unwrap();
        assert_eq!(a.0, Some(8.24));
        let (base, _) = s.baseline(Some("23")).unwrap();
        assert_eq!(base.len(), 1, "baseline gets an anonymous copy");
    }

    #[test]
    fn your_standing_comes_from_the_cgpa_you_entered_never_the_reference() {
        let s = Store::open_in_memory().unwrap();
        // A shortlist of forty, every one with reference academic data.
        let mut regs = BTreeSet::new();
        for i in 0..40 {
            let r = RegNo::parse(&format!("23BCE{:04}", 1000 + i)).unwrap();
            s.upsert_academic(&r, Some(7.0 + i as f64 * 0.05), Some("CSE"), "ref")
                .unwrap();
            regs.insert(r);
        }
        let id = s
            .insert_drive(
                "X",
                None,
                "x.xlsx",
                "H",
                "reg_no_only",
                Some("reg_no"),
                &BTreeSet::new(),
                &regs,
                None,
            )
            .unwrap();
        let identity = build_graph(&s).unwrap();

        // Whoever typed this registration number at setup — the reference
        // data's top student, 8.95 — must not be handed that student's place.
        let mut profile = Profile {
            reg_no: Some("23BCE1039".into()),
            ..Default::default()
        };
        let a = analyse_drive(&s, id, &identity, &profile).unwrap();
        assert!(a.sufficient);
        assert!(
            a.your_percentile.is_none(),
            "no CGPA entered means no standing, never the reference's"
        );

        profile.cgpa = Some(7.0);
        let a = analyse_drive(&s, id, &identity, &profile).unwrap();
        let standing = a.your_percentile.expect("a standing from the entered CGPA");
        assert!(standing.value < 0.05, "placed by 7.0, not by 8.95");
    }

    /// `n` distinct, valid Neo IDs, starting from `from`.
    fn ids(from: usize, n: usize) -> BTreeSet<NeoId> {
        (from..from + n)
            .map(|i| {
                let d: Vec<char> = format!("{i:04}").chars().collect();
                NeoId::parse(&format!("A{}B{}C{}D{}", d[0], d[1], d[2], d[3])).unwrap()
            })
            .collect()
    }

    fn stored(
        s: &Store,
        company: &str,
        hash: &str,
        members: &BTreeSet<NeoId>,
        round: Option<&str>,
    ) -> i64 {
        s.insert_drive(
            company,
            None,
            "f.xlsx",
            hash,
            "neo_id_only",
            Some("neo_id"),
            members,
            &BTreeSet::new(),
            round,
        )
        .unwrap()
    }

    #[test]
    fn a_later_round_is_linked_to_the_one_it_was_drawn_from() {
        let s = Store::open_in_memory().unwrap();
        let first = stored(&s, "Siemens SISW", "a", &ids(0, 40), None);
        let second = stored(&s, "siemens  sisw", "b", &ids(0, 20), None);
        assert_eq!(link_round(&s, second).unwrap(), Some(first));

        // A third round attaches to the second, not back to the first.
        let third = stored(&s, "Siemens SISW", "c", &ids(0, 8), None);
        assert_eq!(link_round(&s, third).unwrap(), Some(second));
    }

    #[test]
    fn old_automatic_names_are_upgraded_and_chosen_ones_are_not() {
        let s = Store::open_in_memory().unwrap();
        let add = |company: &str, file: &str, hash: &str| {
            s.insert_drive(
                company,
                None,
                file,
                hash,
                "neo_id_only",
                Some("neo_id"),
                &ids(0, 5),
                &BTreeSet::new(),
                None,
            )
            .unwrap()
        };
        let auto = add(
            "Infosys s 03rd & 5th oct",
            "Infosys_interviews_03rd & 5th oct.xlsx",
            "a",
        );
        let chosen = add(
            "Infosys Pune",
            "Infosys_interviews_03rd & 5th oct.xlsx",
            "b",
        );

        assert_eq!(upgrade_legacy_names(&s).unwrap(), 1);
        let auto = s.drive(auto).unwrap().unwrap();
        assert_eq!(auto.company, "Infosys");
        assert_eq!(auto.round_label.as_deref(), Some("Interview"));
        let chosen = s.drive(chosen).unwrap().unwrap();
        assert_eq!(chosen.company, "Infosys Pune", "a name someone chose stays");
        assert_eq!(chosen.round_label.as_deref(), Some("Interview"));

        // Once only: a later rename by the student is never undone.
        s.rename_drive(auto.id, "Infosys s 03rd & 5th oct").unwrap();
        assert_eq!(upgrade_legacy_names(&s).unwrap(), 0);
    }

    #[test]
    fn drives_from_before_linking_are_linked_once() {
        let s = Store::open_in_memory().unwrap();
        let r1 = stored(&s, "Infosys", "a", &ids(0, 40), None);
        let r2 = stored(&s, "Infosys", "b", &ids(0, 20), None);
        assert_eq!(link_existing_rounds(&s).unwrap(), 1);
        assert_eq!(s.drive(r2).unwrap().unwrap().parent_drive_id, Some(r1));

        // Separated by the student, and left that way on the next launch.
        s.set_drive_parent(r2, None).unwrap();
        assert_eq!(link_existing_rounds(&s).unwrap(), 0);
        assert_eq!(s.drive(r2).unwrap().unwrap().parent_drive_id, None);
    }

    #[test]
    fn two_roles_at_one_company_are_not_mistaken_for_rounds() {
        let s = Store::open_in_memory().unwrap();
        stored(&s, "Amazon", "a", &ids(0, 40), None);
        // Mostly different students: another role, not a next round.
        let other = stored(&s, "Amazon", "b", &ids(30, 40), None);
        assert_eq!(link_round(&s, other).unwrap(), None);
    }

    #[test]
    fn rounds_need_the_same_company_and_a_name_to_link_by() {
        let s = Store::open_in_memory().unwrap();
        stored(&s, "Zoho", "a", &ids(0, 40), None);
        let elsewhere = stored(&s, "Zluri", "b", &ids(0, 20), None);
        assert_eq!(link_round(&s, elsewhere).unwrap(), None);

        stored(&s, crate::naming::UNNAMED, "c", &ids(100, 40), None);
        let unnamed = stored(&s, crate::naming::UNNAMED, "d", &ids(100, 20), None);
        assert_eq!(link_round(&s, unnamed).unwrap(), None, "nothing to link by");
    }

    #[test]
    fn a_progression_shows_each_round_and_whether_one_is_still_to_come() {
        let s = Store::open_in_memory().unwrap();
        let you = "A0B0C0D5"; // in ids(0, n) for every n > 5
        let profile = Profile {
            neo_id: Some(you.into()),
            ..Default::default()
        };
        let r1 = stored(&s, "Elgi", "a", &ids(0, 40), None);
        let r2 = stored(&s, "Elgi", "b", &ids(0, 20), None);
        link_round(&s, r2).unwrap();

        let p = progression(&s, r1, &profile).unwrap().expect("two rounds");
        let labels: Vec<_> = p.steps.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, ["R1", "R2"]);
        assert!(p.steps.iter().all(|r| r.verdict == Verdict::Shortlisted));
        assert!(p.next_pending, "in the latest round, and it isn't the last");

        // A round the file calls final has nothing after it.
        let last = stored(&s, "Elgi", "c", &ids(0, 6), Some("Final"));
        link_round(&s, last).unwrap();
        let p = progression(&s, r2, &profile).unwrap().unwrap();
        assert_eq!(p.steps.last().unwrap().label, "Final");
        assert!(!p.next_pending);

        // Out of the latest round: nothing pending either.
        let s2 = Store::open_in_memory().unwrap();
        let a = stored(&s2, "Elgi", "a", &ids(0, 40), None);
        let b = stored(&s2, "Elgi", "b", &ids(10, 20), None);
        link_round(&s2, b).unwrap();
        let p = progression(&s2, a, &profile).unwrap().unwrap();
        assert_eq!(p.steps[1].verdict, Verdict::NotShortlisted);
        assert!(!p.next_pending);
    }

    #[test]
    fn a_drive_on_its_own_has_no_progression_or_round_label() {
        let s = Store::open_in_memory().unwrap();
        let only = stored(&s, "Tekion", "a", &ids(0, 10), None);
        assert_eq!(progression(&s, only, &Profile::default()).unwrap(), None);
        assert!(round_labels(&s.drives().unwrap()).is_empty());
    }

    #[test]
    fn evidence_points_at_the_row_a_student_can_check() {
        use crate::store::MemberOrigin;
        let s = Store::open_in_memory().unwrap();
        let members = ids(0, 10);
        let id = stored(&s, "HPE", "a", &members, None);
        s.record_origins(
            id,
            &[MemberOrigin {
                kind: "neo_id".into(),
                value: "A0B0C0D3".into(),
                sheet: "Round 2".into(),
                row: 42,
                column: Some("C".into()),
                header: Some("Neo ID".into()),
            }],
        )
        .unwrap();
        let drive = s.drive(id).unwrap().unwrap();

        let profile = Profile {
            neo_id: Some("a0b0c0d3".into()),
            ..Default::default()
        };
        let e = evidence(&s, &drive, &profile).unwrap();
        assert_eq!(e.key, Some(KeyKind::NeoId));
        assert_eq!(
            e.yours.as_deref(),
            Some("A0B0C0D3"),
            "as stored, not as typed"
        );
        assert_eq!(e.listed, 10);
        let at = e.found_at.expect("where it was");
        assert_eq!(
            (at.sheet.as_str(), at.row, at.column.as_deref()),
            ("Round 2", 42, Some("C"))
        );

        // Someone not on the list has a count to compare against, and no row.
        let absent = Profile {
            neo_id: Some("Z9Y9X9W9".into()),
            ..Default::default()
        };
        let e = evidence(&s, &drive, &absent).unwrap();
        assert_eq!(e.found_at, None);
        assert_eq!(e.listed, 10);
    }

    #[test]
    fn the_stored_verdict_agrees_with_the_rule_in_every_case() {
        let s = Store::open_in_memory().unwrap();
        let members = ids(0, 5);
        let id = stored(&s, "X", "a", &members, None);
        let drive = s.drive(id).unwrap().unwrap();
        for (neo, reg) in [
            (Some("A0B0C0D1"), None),
            (Some("Z9Y9X9W9"), None),
            (None, Some("23BAI0002")),
            (None, None),
            (Some("not an id"), None),
        ] {
            assert_eq!(
                verdict_in_drive(&s, &drive, neo, reg).unwrap(),
                membership_verdict(
                    Some(KeyKind::NeoId),
                    FileShape::NeoIdOnly,
                    neo,
                    reg,
                    &members,
                    &BTreeSet::new()
                ),
                "{neo:?} {reg:?}"
            );
        }
    }

    #[test]
    fn search_refuses_ambiguous_names() {
        let s = Store::open_in_memory().unwrap();
        for (neo, reg) in [("V9H0G6C4", "23AAA0001"), ("C5U6K1E7", "23BBB0002")] {
            let n = Identifier::NeoId(NeoId::parse(neo).unwrap());
            let r = Identifier::RegNo(RegNo::parse(reg).unwrap());
            let k = name::name_key("Naveen");
            s.remember_spelling(&k, "Naveen").unwrap();
            s.add_edge(&n, &r, Confidence::Verified, "f").unwrap();
            s.add_edge(&n, &Identifier::NameKey(k), Confidence::High, "f")
                .unwrap();
        }
        let id = build_graph(&s).unwrap();
        let idx = search_index(&s, &id).unwrap();
        match search_by_name(&idx, "Naveen") {
            MatchOutcome::Ambiguous { candidates } => assert_eq!(candidates.len(), 2),
            other => panic!("must refuse to pick, got {other:?}"),
        }
    }
}
