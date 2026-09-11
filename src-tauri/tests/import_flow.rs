//! End-to-end integration: the critical import → verdict flow.
//!
//! Exercises the whole core the way the application does — parse a real-world
//! file, harvest identity, resolve membership, run analytics — against the
//! anonymised fixtures of all fifteen real shortlists.
//!
//! The tests that matter most here are the negative ones: proving that an
//! unreadable file and a wrong-key file never produce a rejection.

use nankiv_core::analytics::CutoffVerdict;
use nankiv_core::engine::{self, AcademicRow};
use nankiv_core::model::*;
use nankiv_core::parse::{parse_file, FileShape};
use nankiv_core::store::{Profile, Store, StoreError};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Mirrors what the import command does, minus the Tauri plumbing.
fn import(store: &Store, name: &str) -> Result<i64, StoreError> {
    let parsed = parse_file(&fixture(name)).expect("fixture parses");
    let id = store.insert_drive(
        name,
        None,
        name,
        &parsed.content_hash,
        match parsed.shape {
            FileShape::NeoIdOnly => "neo_id_only",
            FileShape::RegNoOnly => "reg_no_only",
            FileShape::Linked => "linked",
            FileShape::Unrecognised => "unrecognised",
        },
        parsed.primary_key.map(|k| match k {
            KeyKind::NeoId => "neo_id",
            KeyKind::RegNo => "reg_no",
        }),
        &parsed.neo_ids,
        &parsed.reg_nos,
        None,
    )?;
    engine::harvest_identity(store, &parsed, name)?;
    Ok(id)
}

fn verdict_for(name: &str, neo: Option<&str>, reg: Option<&str>) -> Verdict {
    let parsed = parse_file(&fixture(name)).expect("fixture parses");
    engine::membership_verdict(
        parsed.primary_key,
        parsed.shape,
        neo,
        reg,
        &parsed.neo_ids,
        &parsed.reg_nos,
    )
}

/// Picks a Neo ID that is genuinely on a given fixture's shortlist.
fn a_member_of(name: &str) -> String {
    let parsed = parse_file(&fixture(name)).expect("fixture parses");
    parsed
        .neo_ids
        .iter()
        .next()
        .expect("fixture has members")
        .as_str()
        .to_string()
}

// ---------------------------------------------------------------------------
// The happy path
// ---------------------------------------------------------------------------

#[test]
fn a_shortlisted_student_gets_a_positive_verdict() {
    let member = a_member_of("siemens_sisw_shortlist_2027.xlsx");
    let v = verdict_for("siemens_sisw_shortlist_2027.xlsx", Some(&member), None);
    assert_eq!(v, Verdict::Shortlisted);
}

#[test]
fn an_absent_student_gets_a_genuine_rejection() {
    // A well-formed Neo ID that is not in this file.
    let parsed = parse_file(&fixture("siemens_sisw_shortlist_2027.xlsx")).unwrap();
    let absent = (0..)
        .map(|i| {
            format!(
                "Z{}Z{}Z{}Z{}",
                i % 10,
                (i / 10) % 10,
                (i / 100) % 10,
                (i / 1000) % 10
            )
        })
        .find(|c| {
            NeoId::parse(c)
                .map(|n| !parsed.neo_ids.contains(&n))
                .unwrap_or(false)
        })
        .expect("an absent id exists");
    assert_eq!(
        verdict_for("siemens_sisw_shortlist_2027.xlsx", Some(&absent), None),
        Verdict::NotShortlisted
    );
}

#[test]
fn the_full_import_flow_produces_a_stored_drive() {
    let s = Store::open_in_memory().unwrap();
    let id = import(&s, "fractal_analytics_shortlist.xlsx").unwrap();
    let d = s.drive(id).unwrap().expect("drive stored");
    assert_eq!(d.total_students, 40);
    assert_eq!(s.drive_neo_ids(id).unwrap().len(), 40);
}

// ---------------------------------------------------------------------------
// The negative paths — the ones that matter
// ---------------------------------------------------------------------------

#[test]
fn an_unreadable_file_never_produces_a_rejection() {
    // TCS/Cognizant. Every possible caller state must yield Undetermined.
    for (neo, reg) in [
        (Some("V9H0G6C4"), Some("23BAI0002")),
        (Some("V9H0G6C4"), None),
        (None, Some("23BAI0002")),
        (None, None),
    ] {
        let v = verdict_for("tcs_shortlist_27th_cognizant.xlsx", neo, reg);
        assert_eq!(
            v,
            Verdict::Undetermined(Undetermined::FileNotUnderstood),
            "for neo={neo:?} reg={reg:?}"
        );
        assert_ne!(v, Verdict::NotShortlisted);
    }
}

#[test]
fn a_reg_keyed_file_is_undetermined_for_a_neo_only_student() {
    // HPE and Deloitte are keyed by registration number. A student who has only
    // saved a Neo ID is simply unanswerable — saying "not shortlisted" would be
    // a fabrication.
    for f in ["hpe_shortlisted_list.xlsx", "deloitte_shortlisted.xlsx"] {
        let v = verdict_for(f, Some("V9H0G6C4"), None);
        assert_eq!(
            v,
            Verdict::Undetermined(Undetermined::KeyKindNotConfigured {
                file_key: KeyKind::RegNo
            }),
            "for {f}"
        );
        assert_ne!(v, Verdict::NotShortlisted, "for {f}");
    }
}

#[test]
fn a_neo_keyed_file_is_undetermined_for_a_reg_only_student() {
    let v = verdict_for("zluri_shortlist_28_07_26.xlsx", None, Some("23BAI0002"));
    assert_eq!(
        v,
        Verdict::Undetermined(Undetermined::KeyKindNotConfigured {
            file_key: KeyKind::NeoId
        })
    );
}

#[test]
fn no_identity_at_all_is_undetermined_everywhere() {
    for f in [
        "elgi_test_shortlist.xlsx",
        "hpe_shortlisted_list.xlsx",
        "tredence_shortlisted_list_1_9_26.xlsx",
    ] {
        assert_eq!(
            verdict_for(f, None, None),
            Verdict::Undetermined(Undetermined::NoIdentityConfigured),
            "for {f}"
        );
    }
}

#[test]
fn every_fixture_yields_a_determinate_answer_for_a_fully_configured_student() {
    // With both identifiers saved, only the genuinely unreadable file should
    // remain undetermined.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut undetermined = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let p = entry.unwrap().path();
        if p.extension().and_then(|e| e.to_str()) != Some("xlsx") {
            continue;
        }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let v = verdict_for(&name, Some("V9H0G6C4"), Some("23BAI0002"));
        if !v.is_determined() {
            undetermined.push(name);
        }
    }
    assert_eq!(
        undetermined,
        vec!["tcs_shortlist_27th_cognizant.xlsx".to_string()],
        "only the foreign-id file should be undetermined"
    );
}

// ---------------------------------------------------------------------------
// Duplicates and rounds
// ---------------------------------------------------------------------------

#[test]
fn re_importing_the_same_shortlist_is_refused() {
    let s = Store::open_in_memory().unwrap();
    import(&s, "siemens_sisw_shortlist_2027.xlsx").unwrap();
    // The duplicate file, under a different name — content hashing catches it.
    let err = import(&s, "siemens_sisw_shortlist_2027_1.xlsx").unwrap_err();
    assert!(matches!(err, StoreError::DuplicateDrive { .. }));
    assert_eq!(
        s.drives().unwrap().len(),
        1,
        "history must not double-count"
    );
}

#[test]
fn round_to_round_comparison_is_exact_regardless_of_coverage() {
    // Pure set algebra on identifiers: this is accurate even when nankiv can
    // name none of the students involved.
    let s = Store::open_in_memory().unwrap();
    let r1 = import(&s, "tredence_shortlisted_list.xlsx").unwrap();
    let r2 = import(&s, "tredence_shortlisted_list_1_9_26.xlsx").unwrap();

    let a = s.drive_neo_ids(r1).unwrap();
    let b = s.drive_neo_ids(r2).unwrap();
    let advanced = a.intersection(&b).count();
    let dropped = a.difference(&b).count();

    assert_eq!(advanced, 819, "everyone in round two came from round one");
    assert_eq!(dropped, 1815 - 819);
}

// ---------------------------------------------------------------------------
// Identity harvesting
// ---------------------------------------------------------------------------

#[test]
fn the_linked_file_teaches_the_graph_verified_links() {
    let s = Store::open_in_memory().unwrap();
    import(&s, "tredence_shortlisted_list_1_9_26.xlsx").unwrap();
    let identity = engine::build_graph(&s).unwrap();

    assert_eq!(identity.conflicted_count, 0, "no contradictions expected");
    assert!(
        identity.neo_to_reg.len() >= 800,
        "expected ~819 neo->reg links, got {}",
        identity.neo_to_reg.len()
    );
    assert!(
        identity.neo_to_name.len() >= 800,
        "names should resolve too, got {}",
        identity.neo_to_name.len()
    );
}

#[test]
fn harvested_identity_improves_a_previously_imported_drive() {
    // The compounding property: import a bare Neo-ID list first, learn identity
    // from a richer file second, and the first drive becomes analysable.
    let s = Store::open_in_memory().unwrap();
    let bare = import(&s, "tredence_shortlisted_list.xlsx").unwrap();

    let before = engine::build_graph(&s).unwrap();
    let named_before = s
        .drive_neo_ids(bare)
        .unwrap()
        .iter()
        .filter(|n| before.name_of_neo(n.as_str()).is_some())
        .count();
    assert_eq!(named_before, 0, "a bare id list names nobody");

    import(&s, "tredence_shortlisted_list_1_9_26.xlsx").unwrap();
    let after = engine::build_graph(&s).unwrap();
    let named_after = s
        .drive_neo_ids(bare)
        .unwrap()
        .iter()
        .filter(|n| after.name_of_neo(n.as_str()).is_some())
        .count();
    assert!(
        named_after >= 800,
        "the earlier drive should now name ~819 students, got {named_after}"
    );
}

// ---------------------------------------------------------------------------
// Analytics through the full stack
// ---------------------------------------------------------------------------

/// Seeds academic records for every student in a fixture, at a fixed CGPA
/// distribution, so analytics can be exercised end to end.
fn seed_academics(s: &Store, fixture_name: &str, make_cgpa: impl Fn(usize) -> f64) {
    let parsed = parse_file(&fixture(fixture_name)).unwrap();
    let identity = engine::build_graph(s).unwrap();
    let rows: Vec<AcademicRow> = parsed
        .neo_ids
        .iter()
        .enumerate()
        .filter_map(|(i, n)| {
            identity.reg_of_neo(n.as_str()).and_then(|r| {
                RegNo::parse(r).map(|reg| AcademicRow {
                    reg_no: reg,
                    name: None,
                    cgpa: Some(make_cgpa(i)),
                    branch: Some("CSE".to_string()),
                })
            })
        })
        .collect();
    engine::ingest_academics(s, &rows, "seed").unwrap();
}

#[test]
fn analytics_are_suppressed_when_coverage_is_too_thin() {
    // The Elgi case through the whole stack: 125 students, essentially none
    // matchable, so nothing statistical may be published.
    let s = Store::open_in_memory().unwrap();
    let id = import(&s, "elgi_test_shortlist.xlsx").unwrap();
    let identity = engine::build_graph(&s).unwrap();
    let a = engine::analyse_drive(&s, id, &identity, &Profile::default()).unwrap();

    assert!(!a.sufficient);
    assert!(a.cgpa.is_none());
    assert!(a.cutoff.is_none());
    assert!(a.branches.is_none());
    assert_eq!(a.total_students, 125);
}

#[test]
fn a_planted_cutoff_is_recovered_end_to_end() {
    let s = Store::open_in_memory().unwrap();
    let id = import(&s, "tredence_shortlisted_list_1_9_26.xlsx").unwrap();
    // Everyone at or above 9.0, one student just below — the Siemens shape.
    seed_academics(&s, "tredence_shortlisted_list_1_9_26.xlsx", |i| {
        if i == 0 {
            8.94
        } else {
            9.0 + (i % 40) as f64 * 0.02
        }
    });
    // A realistic batch baseline, so the comparison has something to work with.
    for v in nankiv_core::analytics::baseline::test_support::batch_like() {
        s.add_baseline("23", Some(v), Some("CSE")).unwrap();
    }

    let identity = engine::build_graph(&s).unwrap();
    let a = engine::analyse_drive(&s, id, &identity, &Profile::default()).unwrap();

    assert!(a.sufficient, "819 students is plenty");
    let cutoff = a.cutoff.expect("a cutoff report");
    match cutoff.verdict.value {
        CutoffVerdict::HardCutoff { threshold, .. } => {
            assert_eq!(threshold, 9.0, "should recover the planted 9.0 bar");
        }
        other => panic!("expected a hard cutoff, got {other:?}"),
    }
    assert!(cutoff.statement.contains("estimate"));
}

#[test]
fn a_batch_shaped_shortlist_reports_no_filter() {
    let s = Store::open_in_memory().unwrap();
    let id = import(&s, "tredence_shortlisted_list_1_9_26.xlsx").unwrap();
    let batch = nankiv_core::analytics::baseline::test_support::batch_like();
    // Draw the shortlist's CGPAs from the batch itself.
    seed_academics(&s, "tredence_shortlisted_list_1_9_26.xlsx", |i| {
        batch[i % batch.len()]
    });
    for v in &batch {
        s.add_baseline("23", Some(*v), Some("CSE")).unwrap();
    }

    let identity = engine::build_graph(&s).unwrap();
    let a = engine::analyse_drive(&s, id, &identity, &Profile::default()).unwrap();
    let cutoff = a.cutoff.expect("a cutoff report");
    assert_eq!(
        cutoff.verdict.value,
        CutoffVerdict::NoCgpaFilter,
        "drawing from the batch must not manufacture a cutoff"
    );
}

#[test]
fn coverage_is_reported_even_when_analytics_are_gated() {
    let s = Store::open_in_memory().unwrap();
    let id = import(&s, "amazon_shortlist_1.xlsx").unwrap();
    let identity = engine::build_graph(&s).unwrap();
    let a = engine::analyse_drive(&s, id, &identity, &Profile::default()).unwrap();
    assert_eq!(a.total_students, 527);
    assert!(a.coverage >= 0.0 && a.coverage <= 1.0);
}

// ---------------------------------------------------------------------------
// Privacy
// ---------------------------------------------------------------------------

#[test]
fn importing_never_stores_contact_details() {
    // Structural guarantee: the academics table has no column for them, so even
    // a sheet full of phone numbers cannot leak into storage.
    let s = Store::open_in_memory().unwrap();
    import(&s, "tredence_shortlisted_list_1_9_26.xlsx").unwrap();

    let conn = s.connection();
    let stmt = conn.prepare("SELECT * FROM academics LIMIT 0").unwrap();
    let cols: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
    for forbidden in ["phone", "email", "dob", "gender", "resume", "mobile"] {
        assert!(
            !cols.iter().any(|c| c.contains(forbidden)),
            "academics table must have no {forbidden} column"
        );
    }
    drop(stmt);
}

#[test]
fn wiping_removes_every_trace() {
    let s = Store::open_in_memory().unwrap();
    import(&s, "fractal_analytics_shortlist.xlsx").unwrap();
    s.add_friend("A", Some("V9H0G6C4"), None, None).unwrap();
    s.wipe().unwrap();

    assert!(s.drives().unwrap().is_empty());
    assert!(s.friends().unwrap().is_empty());
    assert_eq!(s.edge_count().unwrap(), 0);
    assert_eq!(s.academic_count().unwrap(), 0);
    for (_, count) in s.inventory().unwrap() {
        assert_eq!(count, 0);
    }
}
