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
use crate::store::{Profile, Store, StoreError};
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
            Some(id) => Verdict::from_lookup(neo_ids.contains(&id)),
            // The file is keyed by Neo ID and we don't have one for this person.
            // Their absence means nothing.
            None => Verdict::Undetermined(Undetermined::KeyKindNotConfigured {
                file_key: KeyKind::NeoId,
            }),
        },
        KeyKind::RegNo => match subject_reg.and_then(RegNo::parse) {
            Some(id) => Verdict::from_lookup(reg_nos.contains(&id)),
            None => Verdict::Undetermined(Undetermined::KeyKindNotConfigured {
                file_key: KeyKind::RegNo,
            }),
        },
    }
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

    // The student's own CGPA, for the standing figure.
    let your_cgpa = profile
        .reg_no
        .as_deref()
        .or_else(|| {
            profile
                .neo_id
                .as_deref()
                .and_then(|n| identity.reg_of_neo(n))
        })
        .and_then(|r| store.academic(r).ok().flatten())
        .and_then(|(c, _)| c)
        .and_then(sanitise_cgpa);

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
