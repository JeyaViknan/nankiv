//! Exporting a shortlist as a list of names.
//!
//! A shortlist arrives as a column of eight-character codes nobody can read.
//! The export turns it into the list a student actually wants to share: who is
//! on it. Three rules keep that honest.
//!
//! **Every student gets a row.** A name is only written where the match is
//! strong enough to show a person, which on real files is a minority of the
//! list. Dropping the unnamed rows would produce a spreadsheet that looks
//! complete and is not; instead they stay, marked as not identified.
//!
//! **Only confident names.** A `Probable` link can feed a statistic, but putting
//! it in a file that will be forwarded to a group chat would attach the wrong
//! person to a shortlist. Exported names are `High` or `Verified` only.
//!
//! **Nothing the file did not already say, beyond the name.** No CGPA, no
//! branch, and no registration number for a Neo-ID file — those would turn a
//! list of who was shortlisted into a leak of academic records.

use crate::engine::ResolvedIdentity;
use crate::model::KeyKind;
use crate::store::{Store, StoreError};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Xlsx,
    Csv,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Xlsx => "xlsx",
            ExportFormat::Csv => "csv",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportRow {
    pub identifier: String,
    /// Present only when the identity link may name a person.
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportSummary {
    pub path: String,
    pub rows: usize,
    pub named: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("could not write the file: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not build the spreadsheet: {0}")]
    Xlsx(#[from] rust_xlsxwriter::XlsxError),
    #[error("that shortlist is no longer stored")]
    NotFound,
}

/// Collects one row per student on a drive, named where it is safe to.
///
/// Named rows come first, alphabetically, because that is the order someone
/// scanning for a friend reads in; the unnamed ones follow in identifier order
/// so the list is still stable and complete.
pub fn rows_for_drive(
    store: &Store,
    drive_id: i64,
    identity: &ResolvedIdentity,
) -> Result<(KeyKind, Vec<ExportRow>), ExportError> {
    let drive = store.drive(drive_id)?.ok_or(ExportError::NotFound)?;
    let key = match drive.primary_key.as_deref() {
        Some("reg_no") => KeyKind::RegNo,
        _ => KeyKind::NeoId,
    };

    let mut rows: Vec<ExportRow> = match key {
        KeyKind::NeoId => store
            .drive_neo_ids(drive_id)?
            .into_iter()
            .map(|n| ExportRow {
                name: identity.name_of_neo(n.as_str()).map(str::to_string),
                identifier: n.as_str().to_string(),
            })
            .collect(),
        KeyKind::RegNo => store
            .drive_reg_nos(drive_id)?
            .into_iter()
            .map(|r| ExportRow {
                name: identity.name_of_reg(r.as_str()).map(str::to_string),
                identifier: r.as_str().to_string(),
            })
            .collect(),
    };

    rows.sort_by(|a, b| match (&a.name, &b.name) {
        (Some(x), Some(y)) => x
            .to_lowercase()
            .cmp(&y.to_lowercase())
            .then_with(|| a.identifier.cmp(&b.identifier)),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => a.identifier.cmp(&b.identifier),
    });

    Ok((key, rows))
}

fn headers(key: KeyKind) -> [&'static str; 3] {
    let id = match key {
        KeyKind::NeoId => "Neo ID",
        KeyKind::RegNo => "Registration number",
    };
    [id, "Name", "Status"]
}

fn status(row: &ExportRow) -> &'static str {
    if row.name.is_some() {
        "Identified"
    } else {
        "Not identified"
    }
}

/// Ensures the chosen path carries the right extension.
///
/// Save panels do not always append one, and a file called `Tredence shortlist`
/// with no extension opens in nothing.
pub fn with_extension(path: &Path, format: ExportFormat) -> PathBuf {
    let want = format.extension();
    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case(want) => path.to_path_buf(),
        _ => {
            let mut s = path.as_os_str().to_owned();
            s.push(".");
            s.push(want);
            PathBuf::from(s)
        }
    }
}

// ---------------------------------------------------------------------------
// CSV
// ---------------------------------------------------------------------------

/// Escapes one CSV field.
///
/// Two separate hazards. Quoting protects structure: a name like
/// `Gupta, Shresth` must not split into two columns. The leading-character
/// guard protects the person opening it: a spreadsheet treats a cell beginning
/// with `=`, `+`, `-` or `@` as a formula, so a crafted value in a shortlist
/// could execute when the export is opened. Prefixing an apostrophe makes the
/// cell plain text, which is the mitigation OWASP recommends for CSV injection.
pub fn csv_field(value: &str) -> String {
    let guarded = match value.chars().next() {
        Some('=' | '+' | '-' | '@' | '\t' | '\r') => format!("'{value}"),
        _ => value.to_string(),
    };
    if guarded.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", guarded.replace('"', "\"\""))
    } else {
        guarded
    }
}

pub fn to_csv(key: KeyKind, rows: &[ExportRow]) -> String {
    // The byte-order mark is what makes Excel read the file as UTF-8. Without
    // it, any name carrying an accent or a non-Latin character arrives mangled.
    let mut out = String::from('\u{feff}');
    out.push_str(&headers(key).map(csv_field).join(","));
    out.push_str("\r\n");
    for r in rows {
        let fields = [
            csv_field(&r.identifier),
            csv_field(r.name.as_deref().unwrap_or("")),
            csv_field(status(r)),
        ];
        out.push_str(&fields.join(","));
        out.push_str("\r\n");
    }
    out
}

// ---------------------------------------------------------------------------
// Excel
// ---------------------------------------------------------------------------

/// A worksheet name Excel will accept: at most 31 characters, none of the seven
/// it reserves, and not blank.
pub fn sheet_name(company: &str) -> String {
    let cleaned: String = company
        .chars()
        .map(|c| match c {
            '[' | ']' | ':' | '*' | '?' | '/' | '\\' => ' ',
            _ => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('\'');
    let short: String = trimmed.chars().take(31).collect();
    if short.trim().is_empty() {
        "Shortlist".to_string()
    } else {
        short.trim().to_string()
    }
}

pub fn write_xlsx(
    path: &Path,
    company: &str,
    key: KeyKind,
    rows: &[ExportRow],
) -> Result<(), ExportError> {
    use rust_xlsxwriter::{Format, FormatBorder, Workbook};

    let mut book = Workbook::new();
    let sheet = book.add_worksheet();
    sheet.set_name(sheet_name(company))?;

    let header = Format::new()
        .set_bold()
        .set_border_bottom(FormatBorder::Thin);
    let quiet = Format::new().set_font_color(rust_xlsxwriter::Color::Gray);

    for (col, h) in headers(key).iter().enumerate() {
        sheet.write_string_with_format(0, col as u16, *h, &header)?;
    }

    for (i, r) in rows.iter().enumerate() {
        let row = (i + 1) as u32;
        // Written as strings, never as formulas or numbers: an identifier is
        // text, and a registration number read as a number loses nothing today
        // but invites Excel to reformat it tomorrow.
        sheet.write_string(row, 0, &r.identifier)?;
        match &r.name {
            Some(n) => {
                sheet.write_string(row, 1, n)?;
                sheet.write_string(row, 2, status(r))?;
            }
            None => {
                sheet.write_string_with_format(row, 2, status(r), &quiet)?;
            }
        }
    }

    // Ready to use on open: the header stays put while scrolling, every column
    // can be filtered, and nothing is truncated.
    sheet.set_freeze_panes(1, 0)?;
    if !rows.is_empty() {
        sheet.autofilter(0, 0, rows.len() as u32, 2)?;
    }
    sheet.set_column_width(0, 22)?;
    sheet.set_column_width(1, 34)?;
    sheet.set_column_width(2, 16)?;

    book.save(path)?;
    Ok(())
}

/// Writes the export in the requested format and reports what went into it.
pub fn export_drive(
    store: &Store,
    drive_id: i64,
    identity: &ResolvedIdentity,
    path: &Path,
    format: ExportFormat,
) -> Result<ExportSummary, ExportError> {
    let drive = store.drive(drive_id)?.ok_or(ExportError::NotFound)?;
    let (key, rows) = rows_for_drive(store, drive_id, identity)?;
    let target = with_extension(path, format);

    match format {
        ExportFormat::Csv => std::fs::write(&target, to_csv(key, &rows))?,
        ExportFormat::Xlsx => write_xlsx(&target, &drive.company, key, &rows)?,
    }

    Ok(ExportSummary {
        path: target.to_string_lossy().to_string(),
        rows: rows.len(),
        named: rows.iter().filter(|r| r.name.is_some()).count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Confidence, Identifier, NeoId, RegNo};
    use std::collections::BTreeSet;

    fn row(id: &str, name: Option<&str>) -> ExportRow {
        ExportRow {
            identifier: id.to_string(),
            name: name.map(str::to_string),
        }
    }

    #[test]
    fn plain_fields_are_left_alone() {
        assert_eq!(csv_field("Monish D"), "Monish D");
        assert_eq!(csv_field("V9H0G6C4"), "V9H0G6C4");
    }

    #[test]
    fn a_comma_in_a_name_does_not_split_the_column() {
        assert_eq!(csv_field("Gupta, Shresth"), "\"Gupta, Shresth\"");
    }

    #[test]
    fn embedded_quotes_are_doubled() {
        assert_eq!(csv_field(r#"The "Ace""#), r#""The ""Ace""""#);
    }

    #[test]
    fn formula_prefixes_are_neutralised() {
        // A value that opens as a formula in Excel is an injection vector.
        for bad in ["=HYPERLINK(\"x\")", "+1+1", "-2+3", "@SUM(A1)"] {
            let out = csv_field(bad);
            assert!(
                out.starts_with('\'') || out.starts_with("\"'"),
                "{bad} came out as {out}"
            );
        }
    }

    #[test]
    fn csv_carries_a_bom_and_a_header() {
        let csv = to_csv(KeyKind::NeoId, &[row("V9H0G6C4", Some("Monish D"))]);
        assert!(csv.starts_with('\u{feff}'), "Excel needs the BOM for UTF-8");
        let body = csv.trim_start_matches('\u{feff}');
        let mut lines = body.lines();
        assert_eq!(lines.next(), Some("Neo ID,Name,Status"));
        assert_eq!(lines.next(), Some("V9H0G6C4,Monish D,Identified"));
    }

    #[test]
    fn unidentified_rows_are_kept_and_labelled() {
        let csv = to_csv(KeyKind::NeoId, &[row("C5U6K1E7", None)]);
        assert!(csv.contains("C5U6K1E7,,Not identified"));
    }

    #[test]
    fn a_reg_keyed_export_names_its_column_correctly() {
        let csv = to_csv(KeyKind::RegNo, &[]);
        assert!(csv.contains("Registration number,Name,Status"));
    }

    #[test]
    fn extensions_are_added_only_when_missing() {
        let p = Path::new("/tmp/Tredence shortlist");
        assert_eq!(
            with_extension(p, ExportFormat::Csv),
            PathBuf::from("/tmp/Tredence shortlist.csv")
        );
        let q = Path::new("/tmp/list.XLSX");
        assert_eq!(with_extension(q, ExportFormat::Xlsx), q.to_path_buf());
    }

    #[test]
    fn sheet_names_satisfy_excel() {
        assert_eq!(sheet_name("Siemens SISW"), "Siemens SISW");
        assert_eq!(sheet_name("A/B: Test?"), "A B  Test");
        assert_eq!(sheet_name(""), "Shortlist");
        assert!(sheet_name(&"x".repeat(60)).chars().count() <= 31);
    }

    /// A store with one Neo-keyed drive of three students: one verified and
    /// named, one only probably matched, and one unknown.
    fn fixture_store() -> (Store, i64) {
        let s = Store::open_in_memory().unwrap();
        let ids: BTreeSet<NeoId> = ["V9H0G6C4", "C5U6K1E7", "T2D4R9N9"]
            .iter()
            .filter_map(|x| NeoId::parse(x))
            .collect();
        let id = s
            .insert_drive(
                "Tredence",
                None,
                "t.xlsx",
                "H",
                "neo_id_only",
                Some("neo_id"),
                &ids,
                &BTreeSet::new(),
                None,
            )
            .unwrap();

        let named = crate::identity::name::name_key("Monish D");
        s.remember_spelling(&named, "Monish D").unwrap();
        s.add_edge(
            &Identifier::NeoId(NeoId::parse("V9H0G6C4").unwrap()),
            &Identifier::RegNo(RegNo::parse("23BCE1473").unwrap()),
            Confidence::Verified,
            "f",
        )
        .unwrap();
        s.add_edge(
            &Identifier::NeoId(NeoId::parse("V9H0G6C4").unwrap()),
            &Identifier::NameKey(named),
            Confidence::High,
            "f",
        )
        .unwrap();

        let guess = crate::identity::name::name_key("Maybe Person");
        s.remember_spelling(&guess, "Maybe Person").unwrap();
        s.add_edge(
            &Identifier::NeoId(NeoId::parse("C5U6K1E7").unwrap()),
            &Identifier::NameKey(guess),
            Confidence::Probable,
            "fuzzy",
        )
        .unwrap();

        (s, id)
    }

    #[test]
    fn every_student_on_the_drive_gets_a_row() {
        let (s, id) = fixture_store();
        let identity = crate::engine::build_graph(&s).unwrap();
        let (_, rows) = rows_for_drive(&s, id, &identity).unwrap();
        assert_eq!(rows.len(), 3, "no student may be dropped from the export");
    }

    #[test]
    fn only_confident_names_are_exported() {
        // Forwarding a probable match to a group chat attaches the wrong person
        // to a shortlist, so it never reaches the file.
        let (s, id) = fixture_store();
        let identity = crate::engine::build_graph(&s).unwrap();
        let (_, rows) = rows_for_drive(&s, id, &identity).unwrap();

        let named: Vec<_> = rows.iter().filter_map(|r| r.name.as_deref()).collect();
        assert_eq!(named, vec!["Monish D"]);
        assert!(
            rows.iter()
                .all(|r| r.name.as_deref() != Some("Maybe Person")),
            "a probable match must not be exported as a name"
        );
    }

    #[test]
    fn named_rows_come_first() {
        let (s, id) = fixture_store();
        let identity = crate::engine::build_graph(&s).unwrap();
        let (_, rows) = rows_for_drive(&s, id, &identity).unwrap();
        assert!(rows[0].name.is_some());
        assert!(rows[1..].iter().all(|r| r.name.is_none()));
    }

    #[test]
    fn the_export_carries_no_academic_data() {
        let (s, id) = fixture_store();
        s.upsert_academic(
            &RegNo::parse("23BCE1473").unwrap(),
            Some(9.41),
            Some("CSE"),
            "x",
        )
        .unwrap();
        let identity = crate::engine::build_graph(&s).unwrap();
        let (key, rows) = rows_for_drive(&s, id, &identity).unwrap();
        let csv = to_csv(key, &rows);
        for leak in ["9.41", "CSE", "23BCE1473"] {
            assert!(!csv.contains(leak), "export must not contain {leak}");
        }
    }

    #[test]
    fn an_xlsx_export_opens_and_reads_back() {
        // Round-trip through nankiv's own parser: the file is valid, and the
        // identifier column is still found by shape.
        let (s, id) = fixture_store();
        let identity = crate::engine::build_graph(&s).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let summary = export_drive(
            &s,
            id,
            &identity,
            &dir.path().join("Tredence shortlist"),
            ExportFormat::Xlsx,
        )
        .unwrap();

        assert!(summary.path.ends_with(".xlsx"));
        assert_eq!(summary.rows, 3);
        assert_eq!(summary.named, 1);

        let parsed = crate::parse::parse_file(Path::new(&summary.path)).unwrap();
        assert_eq!(parsed.neo_ids.len(), 3);
    }

    #[test]
    fn a_csv_export_reads_back_too() {
        let (s, id) = fixture_store();
        let identity = crate::engine::build_graph(&s).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let summary = export_drive(
            &s,
            id,
            &identity,
            &dir.path().join("list.csv"),
            ExportFormat::Csv,
        )
        .unwrap();
        let parsed = crate::parse::parse_file(Path::new(&summary.path)).unwrap();
        assert_eq!(parsed.neo_ids.len(), 3);
    }

    #[test]
    fn exporting_a_missing_drive_is_an_error() {
        let s = Store::open_in_memory().unwrap();
        let identity = crate::engine::build_graph(&s).unwrap();
        assert!(matches!(
            rows_for_drive(&s, 999, &identity),
            Err(ExportError::NotFound)
        ));
    }
}
