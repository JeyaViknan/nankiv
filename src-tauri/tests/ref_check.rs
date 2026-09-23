//! A maintainer's look at the real sheets, which are not in this repository
//! and never will be: they carry names, CGPAs and contact details.
//!
//! It runs where those files exist and prints what the engine made of them; it
//! skips everywhere else, including CI, which must never have them. Real
//! coverage lives in the anonymised fixtures — parser_regression, import_flow
//! and seeded_flow — which do run everywhere.
//!
//! Point it at files with:
//!
//!   NANKIV_CGPA_SHEET=… NANKIV_ROSTER_SHEET=… \
//!   NANKIV_SHORTLIST=… NANKIV_LINKED_SHORTLIST=… \
//!   cargo test --test ref_check -- --nocapture

use std::path::PathBuf;

/// A path from the environment, or the maintainer's own default.
fn sheet(variable: &str, fallback: PathBuf) -> PathBuf {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .unwrap_or(fallback)
}

#[test]
fn diagnose_real_reference_and_tredence() {
    use nankiv_core::engine::{self};
    use nankiv_core::parse::{self, academic::parse_academic_sheet};
    use nankiv_core::store::{Profile, Store};
    use std::path::Path;

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
    let cgpa = sheet(
        "NANKIV_CGPA_SHEET",
        root.join("Global/name_cgpa_resume.xlsx"),
    );
    let roster = sheet("NANKIV_ROSTER_SHEET", root.join("Global/name_neoid.xlsx"));
    let tredence = sheet(
        "NANKIV_SHORTLIST",
        home.join("Downloads/Tredence shortlisted list.xlsx"),
    );
    let linked = sheet(
        "NANKIV_LINKED_SHORTLIST",
        home.join("Downloads/Tredence shortlisted list_1-9-26.xlsx"),
    );

    if let Some(missing) = [&cgpa, &roster, &tredence, &linked]
        .into_iter()
        .find(|p| !p.exists())
    {
        println!("skipping: {} is not on this machine", missing.display());
        return;
    }
    let tredence = tredence.as_path();
    let linked = linked.as_path();

    let rows = parse_academic_sheet(&cgpa).expect("academic sheet parses");
    let with_cgpa = rows.iter().filter(|r| r.cgpa.is_some()).count();
    println!(
        "ACADEMIC SHEET: {} rows, {} with cgpa",
        rows.len(),
        with_cgpa
    );

    let s = Store::open_in_memory().unwrap();
    println!(
        "ingested: {}",
        engine::ingest_academics(&s, &rows, "cgpa").unwrap()
    );

    for (label, p) in [("roster", &roster), ("linked", &linked.to_path_buf())] {
        if !p.exists() {
            println!("{label}: MISSING");
            continue;
        }
        let parsed = parse::parse_file(p).unwrap();
        let h = engine::harvest_identity(&s, &parsed, label).unwrap();
        println!(
            "{label}: {} neo, verified={} named={}",
            parsed.neo_ids.len(),
            h.verified_links,
            h.named_links
        );
    }

    let parsed = parse::parse_file(tredence).unwrap();
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
    println!(
        "\nTREDENCE: total={} matched={} coverage={:.1}% sufficient={}",
        a.total_students,
        a.matched_students,
        a.coverage * 100.0,
        a.sufficient
    );
    println!("academics stored: {}", s.academic_count().unwrap());
    println!("neo->reg links: {}", identity.neo_to_reg.len());
}
