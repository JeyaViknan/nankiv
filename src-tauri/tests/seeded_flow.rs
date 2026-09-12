//! The out-of-the-box experience.
//!
//! A student installs nankiv, drops the first shortlist of the season, and
//! expects an answer. Before the reference pack shipped they got "we matched 0
//! of 1815 students" and a request to go and find a spreadsheet.

use nankiv_core::engine;
use nankiv_core::parse::{self, FileShape};
use nankiv_core::store::{Profile, Store};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn fresh_install() -> Store {
    let s = Store::open_in_memory().expect("store");
    nankiv_core::reference::seed(&s).expect("seed");
    s
}

#[test]
fn a_fresh_install_already_holds_academic_data() {
    let s = fresh_install();
    assert!(
        s.academic_count().unwrap() > 2000,
        "shipped with {} records",
        s.academic_count().unwrap()
    );
}

#[test]
fn a_fresh_install_resolves_students_to_academic_data() {
    let s = fresh_install();
    let identity = engine::build_graph(&s).unwrap();
    assert!(
        identity.neo_to_reg.len() > 500,
        "only {} students resolvable",
        identity.neo_to_reg.len()
    );
    assert_eq!(
        identity.conflicted_count, 0,
        "pack must not contradict itself"
    );
}

#[test]
fn approximate_matches_never_put_a_name_on_screen() {
    // Coverage is bought with fuzzy matching, and that is only acceptable
    // because a fuzzy link can feed a statistic but never label a person.
    let s = fresh_install();
    let identity = engine::build_graph(&s).unwrap();
    for (neo, (_, conf)) in &identity.neo_to_reg {
        if *conf == nankiv_core::model::Confidence::Probable {
            assert!(
                identity.name_of_neo(neo).is_none()
                    || identity.confidence_of_neo(neo).can_name_person(),
                "{neo} is only probably matched yet would be named"
            );
        }
    }
}

#[test]
fn wiping_restores_the_bundled_data() {
    let s = fresh_install();
    s.wipe().unwrap();
    assert_eq!(s.academic_count().unwrap(), 0);
    nankiv_core::reference::seed(&s).unwrap();
    assert!(s.academic_count().unwrap() > 2000, "should come back");
}

#[test]
fn a_students_own_import_outranks_the_bundled_guess() {
    // A file printing both identifiers in one row is fact; the pack's name
    // matching is inference. Fact must win.
    let s = fresh_install();
    let parsed = parse::parse_file(&fixture("tredence_shortlisted_list_1_9_26.xlsx")).unwrap();
    assert_eq!(parsed.shape, FileShape::Linked);
    engine::harvest_identity(&s, &parsed, "tredence").unwrap();

    let identity = engine::build_graph(&s).unwrap();
    let verified = identity
        .neo_to_reg
        .values()
        .filter(|(_, c)| *c == nankiv_core::model::Confidence::Verified)
        .count();
    assert!(
        verified >= 800,
        "expected ~819 verified links, got {verified}"
    );
    assert_eq!(identity.conflicted_count, 0);
}

#[test]
fn the_first_import_of_the_season_produces_an_analysis() {
    let s = fresh_install();
    let parsed = parse::parse_file(&fixture("tredence_shortlisted_list.xlsx")).unwrap();
    let id = s
        .insert_drive(
            "Tredence",
            None,
            "t.xlsx",
            &parsed.content_hash,
            "neo_id_only",
            Some("neo_id"),
            &parsed.neo_ids,
            &parsed.reg_nos,
            None,
        )
        .unwrap();
    engine::harvest_identity(&s, &parsed, "tredence").unwrap();

    let identity = engine::build_graph(&s).unwrap();
    let a = engine::analyse_drive(&s, id, &identity, &Profile::default()).unwrap();

    // The fixtures are anonymised, so their synthetic ids cannot match the real
    // pack. What must hold is that the machinery runs and reports honestly.
    assert_eq!(a.total_students, 1815);
    assert!(a.coverage >= 0.0 && a.coverage <= 1.0);
    if a.sufficient {
        assert!(a.cutoff.is_some());
    }
}
