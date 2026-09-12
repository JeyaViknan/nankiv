//! Reference-sheet parsing for academic data.
//!
//! **Minimisation happens here, by omission.** The real reference sheet carries
//! personal email addresses, phone numbers, gender, date of birth, resume links
//! and 10th/12th marks alongside the CGPA. This parser reads four things —
//! registration number, name, CGPA, branch — and there is no code path that
//! extracts the rest. They cannot reach storage because they are never read.

use super::{cell_text, ParseError, MAX_FILE_BYTES};
use crate::engine::AcademicRow;
use crate::model::{sanitise_cgpa, RegNo};
use calamine::{open_workbook_auto, Data, Reader};
use std::path::Path;

/// Column headers that indicate a CGPA column. Header text is a *hint* here,
/// unlike identifier detection: a bare number has no distinguishing shape, so
/// there is nothing else to go on.
const CGPA_HINTS: [&str; 5] = ["cgpa", "gpa", "cg pa", "grade point", "current cgpa"];
const BRANCH_HINTS: [&str; 6] = [
    "brach",
    "branch",
    "specialisation",
    "specialization",
    "programme",
    "program",
];

/// Reads a sheet of academic records. Returns an empty vector when the file
/// carries no CGPA column, which tells the caller to treat it as a roster.
pub fn parse_academic_sheet(path: &Path) -> Result<Vec<AcademicRow>, ParseError> {
    let meta = std::fs::metadata(path).map_err(|e| ParseError::Io(e.to_string()))?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(ParseError::TooLarge(meta.len()));
    }
    // A reference sheet exported from Google Sheets arrives as CSV as often as
    // not, so it gets the same treatment as a shortlist.
    let sheets: Vec<Vec<Vec<Data>>> = if super::csv::is_csv(path) {
        let text = std::fs::read_to_string(path).map_err(|e| ParseError::Io(e.to_string()))?;
        vec![super::csv::parse_grid(&text)]
    } else {
        let mut wb = open_workbook_auto(path).map_err(|e| ParseError::Open(e.to_string()))?;
        let names: Vec<String> = wb.sheet_names().to_vec();
        names
            .iter()
            .filter_map(|n| wb.worksheet_range(n).ok())
            .map(|range| range.rows().map(|r| r.to_vec()).collect())
            .collect()
    };

    let mut out: Vec<AcademicRow> = Vec::new();

    for rows in &sheets {
        let rows = rows.clone();
        if rows.len() < 2 {
            continue;
        }

        let header: Vec<String> = rows[0]
            .iter()
            .map(|c| cell_text(c).to_lowercase())
            .collect();
        let find = |hints: &[&str]| -> Option<usize> {
            header
                .iter()
                .position(|h| hints.iter().any(|x| h.contains(x)))
        };

        let cgpa_col = find(&CGPA_HINTS);
        let branch_col = find(&BRANCH_HINTS);

        // Registration column is found by value shape, as everywhere else.
        let reg_col = (0..rows[0].len().max(1)).max_by_key(|&i| {
            rows.iter()
                .filter(|r| {
                    r.get(i)
                        .map(|c| RegNo::parse(&cell_text(c)).is_some())
                        .unwrap_or(false)
                })
                .count()
        });
        let Some(reg_col) = reg_col else { continue };
        let reg_hits = rows
            .iter()
            .filter(|r| {
                r.get(reg_col)
                    .map(|c| RegNo::parse(&cell_text(c)).is_some())
                    .unwrap_or(false)
            })
            .count();
        if reg_hits < 2 {
            continue;
        }

        // The name column, chosen by how *distinct* its values are.
        //
        // `Name`, `Gender`, `Degree` and `Campus` all pass the "looks like a
        // name" test on every row of the real reference sheet, so counting
        // matches ties four ways and the tie-break silently picked the last
        // column — assigning every student the name "vellore" and killing the
        // name bridge entirely. Names are nearly all different; categories are
        // nearly all the same.
        let name_col = (0..rows[0].len())
            .filter(|i| Some(*i) != branch_col && *i != reg_col)
            .filter(|&i| {
                let hits = rows
                    .iter()
                    .filter(|r| {
                        r.get(i)
                            .map(|c| super::shape::looks_like_name(&cell_text(c)))
                            .unwrap_or(false)
                    })
                    .count();
                hits * 2 >= rows.len()
            })
            .map(|i| {
                let values: Vec<String> = rows
                    .iter()
                    .filter_map(|r| r.get(i))
                    .map(cell_text)
                    .collect();
                (i, super::shape::distinctness(&values))
            })
            .filter(|(_, d)| *d >= 0.35)
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i);

        for row in rows.iter().skip(1) {
            let Some(reg) = row
                .get(reg_col)
                .map(cell_text)
                .and_then(|s| RegNo::parse(&s))
            else {
                continue;
            };
            let cgpa = cgpa_col
                .and_then(|i| row.get(i))
                .map(cell_text)
                .and_then(|s| s.parse::<f64>().ok())
                .and_then(sanitise_cgpa);
            let branch = branch_col
                .and_then(|i| row.get(i))
                .map(cell_text)
                .filter(|s| !s.is_empty());
            let name = name_col
                .and_then(|i| row.get(i))
                .map(cell_text)
                .filter(|s| super::shape::looks_like_name(s));

            out.push(AcademicRow {
                reg_no: reg,
                name,
                cgpa,
                branch,
            });
        }

        // Only a sheet with a CGPA column counts as an academic sheet.
        if cgpa_col.is_none() {
            out.clear();
        } else {
            break;
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cgpa_hints_cover_the_real_header() {
        // The reference sheet's header is exactly "CGPA".
        assert!(CGPA_HINTS.iter().any(|h| "cgpa".contains(h)));
    }

    #[test]
    fn branch_hints_cover_the_misspelled_real_header() {
        // The real sheet spells it "Brach".
        assert!(BRANCH_HINTS.iter().any(|h| "brach".contains(h)));
        assert!(BRANCH_HINTS.iter().any(|h| "branch".contains(h)));
    }

    #[test]
    fn a_missing_file_is_an_error() {
        let r = parse_academic_sheet(Path::new("/nonexistent/nope.xlsx"));
        assert!(r.is_err());
    }
}
