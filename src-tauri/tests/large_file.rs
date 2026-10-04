//! A shortlist of five thousand students, end to end.
//!
//! The largest real list this season had 1,552 names; a mass-recruiter's
//! first round can run to several thousand. This builds a five-thousand-row
//! workbook and runs it through everything an import does — reading it,
//! storing it with where every student sat, answering a student near the
//! end of it, finding the evidence, analysing it — and holds the whole thing
//! to a time a student would not notice.

use nankiv_core::engine;
use nankiv_core::model::KeyKind;
use nankiv_core::parse;
use nankiv_core::store::{MemberOrigin, Profile, Store};
use std::time::{Duration, Instant};

const STUDENTS: usize = 5_000;

/// A distinct, valid Neo ID and registration number for each student.
fn ids(i: usize) -> (String, String) {
    let l = |n: usize| (b'A' + (n % 26) as u8) as char;
    let d: Vec<char> = format!("{i:04}").chars().collect();
    (
        format!(
            "{}{}{}{}{}{}{}{}",
            l(i / 26 / 26),
            d[0],
            l(i / 26),
            d[1],
            l(i),
            d[2],
            'K',
            d[3]
        ),
        format!("23BCE{i:04}"),
    )
}

fn workbook(path: &std::path::Path) {
    use rust_xlsxwriter::Workbook;
    let mut book = Workbook::new();
    let sheet = book.add_worksheet();
    sheet
        .write(0, 0, "Mass recruiter — shortlisted for online test")
        .unwrap();
    for (c, h) in ["S.No", "Name", "Neo ID", "Registration Number"]
        .iter()
        .enumerate()
    {
        sheet.write(2, c as u16, *h).unwrap();
    }
    for i in 0..STUDENTS {
        let (neo, reg) = ids(i);
        let r = 3 + i as u32;
        sheet.write(r, 0, i as u32 + 1).unwrap();
        sheet.write(r, 1, "A Student").unwrap();
        sheet.write(r, 2, neo).unwrap();
        sheet.write(r, 3, reg).unwrap();
    }
    book.save(path).unwrap();
}

#[test]
fn five_thousand_students_are_read_stored_and_answered_quickly() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("mass recruiter test shortlist.xlsx");
    workbook(&path);

    let start = Instant::now();
    let parsed = parse::parse_file(&path).unwrap();
    let read = start.elapsed();
    assert_eq!(parsed.neo_ids.len(), STUDENTS);
    assert_eq!(parsed.reg_nos.len(), STUDENTS);
    assert_eq!(parsed.origins.len(), parsed.rows.len());

    let s = Store::open_in_memory().unwrap();
    let start = Instant::now();
    let id = s
        .insert_drive(
            "Mass Recruiter",
            None,
            "mass recruiter test shortlist.xlsx",
            &parsed.content_hash,
            "linked",
            Some("neo_id"),
            &parsed.neo_ids,
            &parsed.reg_nos,
            Some("Test"),
        )
        .unwrap();
    let origins: Vec<MemberOrigin> = parsed
        .rows
        .iter()
        .zip(&parsed.origins)
        .filter_map(|(row, o)| {
            Some(MemberOrigin {
                kind: "neo_id".into(),
                value: row.neo_id.as_ref()?.as_str().to_string(),
                sheet: o.sheet.clone(),
                row: o.row,
                column: o.neo_column.as_ref().map(|c| c.letter.clone()),
                header: o.neo_column.as_ref().and_then(|c| c.header.clone()),
            })
        })
        .collect();
    s.record_origins(id, &origins).unwrap();
    engine::harvest_identity(&s, &parsed, "mass recruiter test shortlist.xlsx").unwrap();
    let stored = start.elapsed();

    // A student near the end of the list.
    let (neo, _) = ids(4_998);
    let profile = Profile {
        neo_id: Some(neo.clone()),
        ..Default::default()
    };
    let start = Instant::now();
    let drive = s.drive(id).unwrap().unwrap();
    let verdict = engine::verdict_in_drive(&s, &drive, Some(&neo), None).unwrap();
    let evidence = engine::evidence(&s, &drive, &profile).unwrap();
    let identity = engine::build_graph(&s).unwrap();
    let analysis = engine::analyse_drive(&s, id, &identity, &profile).unwrap();
    let answered = start.elapsed();

    assert_eq!(verdict, nankiv_core::model::Verdict::Shortlisted);
    assert_eq!(evidence.key, Some(KeyKind::NeoId));
    let at = evidence.found_at.expect("where the student sat");
    assert_eq!((at.row, at.column.as_deref()), (4_998 + 4, Some("C")));
    assert_eq!(at.header.as_deref(), Some("Neo ID"));
    assert_eq!(analysis.total_students, STUDENTS);

    println!("read {read:?}, stored {stored:?}, answered {answered:?}");
    // Generous enough for a debug build on a slow CI runner; a release build
    // on a laptop is several times faster.
    let budget = Duration::from_secs(20);
    assert!(
        read + stored + answered < budget,
        "{read:?} + {stored:?} + {answered:?}"
    );
}
