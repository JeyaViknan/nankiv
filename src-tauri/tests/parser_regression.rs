//! Parser regression tests against every real-world shortlist format.
//!
//! The fixtures are structure-preserving anonymisations of the fifteen real
//! files (see `scripts/make_fixtures.py`). Identifiers and names are synthetic;
//! sheet layout, column order, header spelling and blank sheets are exactly as
//! the companies sent them.
//!
//! Any parser change that alters one of these counts fails loudly, which is the
//! point: these numbers were measured against the real files.

use nankiv_core::model::KeyKind;
use nankiv_core::parse::{parse_file, FileShape};
use std::path::{Path, PathBuf};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn parse(name: &str) -> nankiv_core::parse::ParsedFile {
    let p = fixture(name);
    assert!(
        p.exists(),
        "missing fixture {name} — run scripts/make_fixtures.py"
    );
    parse_file(&p).unwrap_or_else(|e| panic!("failed to parse {name}: {e}"))
}

/// (fixture, expected neo ids, expected reg nos, expected shape)
const CORPUS: &[(&str, usize, usize, FileShape)] = &[
    // --- Shape A: Neo ID only (ten of fifteen real files) ---
    ("elgi_test_shortlist.xlsx", 125, 0, FileShape::NeoIdOnly),
    (
        "fractal_analytics_shortlist.xlsx",
        40,
        0,
        FileShape::NeoIdOnly,
    ),
    (
        "siemens_sisw_shortlist_2027.xlsx",
        166,
        0,
        FileShape::NeoIdOnly,
    ),
    (
        "siemens_sisw_shortlist_2027_1.xlsx",
        166,
        0,
        FileShape::NeoIdOnly,
    ),
    ("amazon_shortlist_1.xlsx", 527, 0, FileShape::NeoIdOnly),
    (
        "zluri_shortlist_28_07_26.xlsx",
        400,
        0,
        FileShape::NeoIdOnly,
    ),
    (
        "tekion_additonal_shortlist.xlsx",
        1107,
        0,
        FileShape::NeoIdOnly,
    ),
    (
        "tredence_shortlisted_list.xlsx",
        1815,
        0,
        FileShape::NeoIdOnly,
    ),
    // Its only sheet is named `Sheet2`, not `Sheet1`.
    (
        "blackrock_test_shortlist.xlsx",
        267,
        0,
        FileShape::NeoIdOnly,
    ),
    // Has a leading serial column and a sparse "Batch" column.
    ("au_small_shortlist.xlsx", 36, 0, FileShape::NeoIdOnly),
    // Has a trailing "ROLE" column of repeated text.
    (
        "epsilon_interview_shortlist_with_neo_id.xlsx",
        88,
        0,
        FileShape::NeoIdOnly,
    ),
    // --- Shape B: registration number keyed ---
    // Counts are *distinct* identifiers: both of these files contain a handful
    // of repeated rows, which the parser deduplicates.
    ("hpe_shortlisted_list.xlsx", 0, 872, FileShape::RegNoOnly),
    ("deloitte_shortlisted.xlsx", 0, 1140, FileShape::RegNoOnly),
    // --- Shape C: both keys in one row — the identity donor ---
    (
        "tredence_shortlisted_list_1_9_26.xlsx",
        819,
        819,
        FileShape::Linked,
    ),
];

#[test]
fn every_real_format_parses_to_the_expected_counts() {
    for (name, neo, reg, shape) in CORPUS {
        let f = parse(name);
        assert_eq!(f.neo_ids.len(), *neo, "{name}: neo id count");
        assert_eq!(f.reg_nos.len(), *reg, "{name}: reg no count");
        assert_eq!(f.shape, *shape, "{name}: shape");
    }
}

#[test]
fn the_foreign_reference_id_file_is_rejected_not_silently_emptied() {
    // TCS/Cognizant uses `REFERENCE_ID` values like `CT20264996884`, which
    // belong to no identifier space this application knows. Returning an empty
    // shortlist would render as "you were not shortlisted" — the single worst
    // bug available in this product.
    let f = parse("tcs_shortlist_27th_cognizant.xlsx");
    assert_eq!(f.shape, FileShape::Unrecognised);
    assert!(!f.shape.is_usable());
    assert_eq!(f.primary_key, None);
    assert!(f.neo_ids.is_empty() && f.reg_nos.is_empty());
    // It must still report what it saw, so the interface can explain itself.
    assert!(
        f.observed_headers
            .iter()
            .any(|h| h.to_uppercase().contains("REFERENCE")),
        "should surface the header it found, got {:?}",
        f.observed_headers
    );
}

#[test]
fn a_non_default_sheet_name_is_not_a_problem() {
    // BlackRock's only sheet is named `Sheet2`, not `Sheet1`. A parser that
    // looked for a sheet by name — rather than iterating all of them — would
    // return nothing here and the student would read that as a rejection.
    let f = parse("blackrock_test_shortlist.xlsx");
    assert_eq!(f.neo_ids.len(), 267);
    assert_eq!(f.sheet_names, vec!["Sheet2".to_string()]);
}

#[test]
fn trailing_empty_sheets_do_not_break_detection() {
    let f = parse("siemens_sisw_shortlist_2027.xlsx");
    assert_eq!(f.neo_ids.len(), 166);
    assert!(
        f.sheet_names.len() >= 3,
        "fixture should retain the empty trailing sheets"
    );
}

#[test]
fn header_spelling_variations_are_irrelevant() {
    // Across the corpus the same concept is spelled `Neo ID`, `NEO ID`,
    // `Neo Id ` and `NEO ID `. Detection is by value shape, so all parse alike.
    for name in [
        "elgi_test_shortlist.xlsx",
        "siemens_sisw_shortlist_2027.xlsx",
        "tekion_additonal_shortlist.xlsx",
        "zluri_shortlist_28_07_26.xlsx",
    ] {
        let f = parse(name);
        assert_eq!(f.primary_key, Some(KeyKind::NeoId), "{name}");
        assert!(!f.neo_ids.is_empty(), "{name}");
    }
}

#[test]
fn serial_and_metadata_columns_are_not_mistaken_for_identifiers() {
    // AU Small: Sno | Batch | Neo ID. Epsilon: NEO ID | ROLE.
    let au = parse("au_small_shortlist.xlsx");
    assert_eq!(au.neo_ids.len(), 36);
    assert_eq!(au.reg_nos.len(), 0);

    let eps = parse("epsilon_interview_shortlist_with_neo_id.xlsx");
    assert_eq!(eps.neo_ids.len(), 88);
    assert_eq!(eps.reg_nos.len(), 0);
}

#[test]
fn the_linked_file_yields_paired_rows_for_identity_harvesting() {
    // This is the file that donates verified Neo ID <-> Reg No links.
    let f = parse("tredence_shortlisted_list_1_9_26.xlsx");
    let paired = f
        .rows
        .iter()
        .filter(|r| r.neo_id.is_some() && r.reg_no.is_some())
        .count();
    assert_eq!(paired, 819, "every row should carry both keys");
    let named = f.rows.iter().filter(|r| r.name.is_some()).count();
    assert!(named > 800, "names should also be picked up, got {named}");
}

#[test]
fn identical_files_produce_identical_content_hashes() {
    // The two Siemens files are duplicates. Import must detect that rather than
    // double-counting the drive in history and streaks.
    let a = parse("siemens_sisw_shortlist_2027.xlsx");
    let b = parse("siemens_sisw_shortlist_2027_1.xlsx");
    assert_eq!(a.content_hash, b.content_hash);
}

#[test]
fn different_files_produce_different_content_hashes() {
    let a = parse("elgi_test_shortlist.xlsx");
    let b = parse("fractal_analytics_shortlist.xlsx");
    assert_ne!(a.content_hash, b.content_hash);
}

#[test]
fn the_larger_tredence_file_is_a_superset_of_the_linked_one() {
    // A real relationship preserved by the anonymiser: the same student maps to
    // the same synthetic id across files, so cross-file identity still works.
    let full = parse("tredence_shortlisted_list.xlsx");
    let linked = parse("tredence_shortlisted_list_1_9_26.xlsx");
    let overlap = linked.neo_ids.intersection(&full.neo_ids).count();
    assert_eq!(
        overlap,
        linked.neo_ids.len(),
        "every id in the linked file should appear in the full list"
    );
}

#[test]
fn cross_company_overlap_survives_anonymisation() {
    // Neo IDs are stable per student across drives — that is the premise the
    // whole product rests on. The fixtures must preserve it or the identity
    // tests below are meaningless.
    let tredence = parse("tredence_shortlisted_list.xlsx");
    let amazon = parse("amazon_shortlist_1.xlsx");
    let overlap = tredence.neo_ids.intersection(&amazon.neo_ids).count();
    assert!(
        overlap > 100,
        "expected substantial cross-company overlap, got {overlap}"
    );
}

#[test]
fn a_missing_file_is_an_error_not_a_panic() {
    let r = parse_file(&fixture("does_not_exist.xlsx"));
    assert!(r.is_err());
}

#[test]
fn a_non_spreadsheet_is_rejected_cleanly() {
    let dir = std::env::temp_dir().join("nankiv_parse_test");
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("not_a_spreadsheet.xlsx");
    std::fs::write(&p, b"this is plainly not a spreadsheet").unwrap();
    let r = parse_file(&p);
    assert!(r.is_err(), "garbage input must not parse");
    let _ = std::fs::remove_file(&p);
}

#[test]
fn every_fixture_parses_without_panicking() {
    // Broad safety net: whatever else changes, no real-world file may crash the
    // parser. A spreadsheet is untrusted input.
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).expect("fixtures directory") {
        let p = entry.unwrap().path();
        if p.extension().and_then(|e| e.to_str()) != Some("xlsx") {
            continue;
        }
        let _ = parse_file(&p);
        count += 1;
    }
    assert_eq!(count, 15, "expected all fifteen fixtures present");
}
