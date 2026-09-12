//! Spreadsheet parsing.
//!
//! Detection is **format-driven, never header-driven**. Across the fifteen real
//! shortlists in the sample corpus the same concept appears as `Neo ID`,
//! `NEO ID`, `Neo Id `, `NEO ID ` and, in one file, as `NEO ID` beside
//! `Register Number` — while Deloitte calls a registration number `USN`. A
//! synonym list would break on the next company. The identifier *shape* has been
//! invariant across 5,339 samples, so that is what we trust.

use crate::model::{Identifier, KeyKind, NeoId, RegNo};
use calamine::{open_workbook_auto, Data, Reader};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::Path;

pub mod academic;
pub mod csv;
pub mod shape;
pub use shape::FileShape;

/// Hard caps. A spreadsheet is untrusted input; these prevent a decompression
/// bomb from exhausting memory.
const MAX_ROWS: usize = 200_000;
const MAX_COLS: usize = 256;
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("could not open the file: {0}")]
    Open(String),
    #[error("the file is empty")]
    Empty,
    #[error("the file is too large ({0} bytes)")]
    TooLarge(u64),
    #[error("could not read the file: {0}")]
    Io(String),
}

/// One row's worth of identifiers found together.
///
/// When a row carries more than one identifier type these become *verified*
/// links in the identity graph — this is how a single file donated 819 exact
/// Neo ID ↔ registration number pairs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedRow {
    pub neo_id: Option<NeoId>,
    pub reg_no: Option<RegNo>,
    pub name: Option<String>,
}

impl ParsedRow {
    pub fn is_empty(&self) -> bool {
        self.neo_id.is_none() && self.reg_no.is_none()
    }

    /// Identifiers in this row, for graph edge creation.
    pub fn identifiers(&self) -> Vec<Identifier> {
        let mut v = Vec::new();
        if let Some(n) = &self.neo_id {
            v.push(Identifier::NeoId(n.clone()));
        }
        if let Some(r) = &self.reg_no {
            v.push(Identifier::RegNo(r.clone()));
        }
        v
    }
}

/// The result of reading a spreadsheet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFile {
    pub rows: Vec<ParsedRow>,
    pub shape: FileShape,
    /// Distinct Neo IDs, deduplicated.
    pub neo_ids: BTreeSet<NeoId>,
    /// Distinct registration numbers, deduplicated.
    pub reg_nos: BTreeSet<RegNo>,
    /// Which key the file is predominantly organised by.
    pub primary_key: Option<KeyKind>,
    /// SHA-256 over normalised identifier content — stable across re-downloads
    /// and renames, so duplicate imports are detected reliably.
    pub content_hash: String,
    /// Header text found above the elected columns. Used for the "we didn't
    /// understand this file" message only, never for detection.
    pub observed_headers: Vec<String>,
    pub sheet_names: Vec<String>,
}

impl ParsedFile {
    pub fn student_count(&self) -> usize {
        match self.primary_key {
            Some(KeyKind::NeoId) => self.neo_ids.len(),
            Some(KeyKind::RegNo) => self.reg_nos.len(),
            None => 0,
        }
    }
}

/// Reads a spreadsheet from disk.
pub fn parse_file(path: &Path) -> Result<ParsedFile, ParseError> {
    let meta = std::fs::metadata(path).map_err(|e| ParseError::Io(e.to_string()))?;
    if meta.len() > MAX_FILE_BYTES {
        return Err(ParseError::TooLarge(meta.len()));
    }

    // A CSV has no workbook structure, so it is read separately and presented
    // to the rest of the pipeline as a single sheet.
    let sheets: Vec<(String, Vec<Vec<Data>>)> = if csv::is_csv(path) {
        let text = std::fs::read_to_string(path).map_err(|e| ParseError::Io(e.to_string()))?;
        let grid = csv::parse_grid(&text);
        if grid.is_empty() {
            return Err(ParseError::Empty);
        }
        vec![("Sheet1".to_string(), grid)]
    } else {
        let mut workbook =
            open_workbook_auto(path).map_err(|e| ParseError::Open(friendly_open_error(&e)))?;
        let names: Vec<String> = workbook.sheet_names().to_vec();
        if names.is_empty() {
            return Err(ParseError::Empty);
        }
        names
            .into_iter()
            .filter_map(|name| {
                let range = workbook.worksheet_range(&name).ok()?;
                let grid: Vec<Vec<Data>> = range
                    .rows()
                    .take(MAX_ROWS)
                    .map(|r| r.iter().take(MAX_COLS).cloned().collect())
                    .collect();
                Some((name, grid))
            })
            .collect()
    };

    let sheet_names: Vec<String> = sheets.iter().map(|(n, _)| n.clone()).collect();
    let mut all_rows: Vec<ParsedRow> = Vec::new();
    let mut headers: Vec<String> = Vec::new();

    // Every sheet is scanned. One sampled file keeps its data on `Sheet2` with
    // `Sheet1` empty; another carries two empty trailing sheets. Assuming the
    // first sheet would silently return nothing for both.
    for (_name, grid) in &sheets {
        if grid.is_empty() {
            continue;
        }
        let grid = grid.as_slice();

        let layout = shape::elect_columns(grid);
        if layout.is_barren() {
            // Remember the header text so an unreadable file can explain itself.
            if let Some(first) = grid.first() {
                for cell in first.iter().take(8) {
                    let t = cell_text(cell);
                    if !t.is_empty() {
                        headers.push(t);
                    }
                }
            }
            continue;
        }

        for row in grid {
            let parsed = layout.extract(row);
            if !parsed.is_empty() {
                all_rows.push(parsed);
            }
        }
        for h in layout.header_labels(grid) {
            if !headers.contains(&h) {
                headers.push(h);
            }
        }
    }

    let mut neo_ids = BTreeSet::new();
    let mut reg_nos = BTreeSet::new();
    for r in &all_rows {
        if let Some(n) = &r.neo_id {
            neo_ids.insert(n.clone());
        }
        if let Some(g) = &r.reg_no {
            reg_nos.insert(g.clone());
        }
    }

    let primary_key = if neo_ids.len() >= reg_nos.len() && !neo_ids.is_empty() {
        Some(KeyKind::NeoId)
    } else if !reg_nos.is_empty() {
        Some(KeyKind::RegNo)
    } else {
        None
    };

    let shape = FileShape::classify(&neo_ids, &reg_nos, &all_rows);
    let content_hash = hash_content(&neo_ids, &reg_nos);

    Ok(ParsedFile {
        rows: all_rows,
        shape,
        neo_ids,
        reg_nos,
        primary_key,
        content_hash,
        observed_headers: headers,
        sheet_names,
    })
}

/// Stable hash over the sorted identifier set. Independent of row order,
/// filename, and any extra columns a company happens to include.
fn hash_content(neo: &BTreeSet<NeoId>, reg: &BTreeSet<RegNo>) -> String {
    let mut h = Sha256::new();
    for n in neo {
        h.update(n.as_str().as_bytes());
        h.update(b"\x1f");
    }
    h.update(b"\x1e");
    for r in reg {
        h.update(r.as_str().as_bytes());
        h.update(b"\x1f");
    }
    format!("{:x}", h.finalize())
}

fn friendly_open_error(e: &calamine::Error) -> String {
    let raw = e.to_string();
    if raw.contains("Zip") || raw.contains("zip") {
        "the file does not look like a valid spreadsheet".to_string()
    } else {
        raw
    }
}

/// Renders a cell as trimmed text. Numeric cells are rendered without a decimal
/// tail so that an ID stored as a number does not become `23000.0`.
pub fn cell_text(cell: &Data) -> String {
    match cell {
        Data::String(s) => s.trim().to_string(),
        Data::Int(i) => i.to_string(),
        Data::Float(f) => {
            if (f.fract()).abs() < f64::EPSILON {
                format!("{}", *f as i64)
            } else {
                format!("{f}")
            }
        }
        Data::Bool(b) => b.to_string(),
        Data::DateTime(d) => d.to_string(),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.trim().to_string(),
        Data::Error(_) | Data::Empty => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_cells_do_not_grow_decimal_tails() {
        assert_eq!(cell_text(&Data::Float(23.0)), "23");
        assert_eq!(cell_text(&Data::Int(42)), "42");
        assert_eq!(cell_text(&Data::Float(8.82)), "8.82");
        assert_eq!(cell_text(&Data::Empty), "");
    }

    #[test]
    fn content_hash_ignores_ordering() {
        let a: BTreeSet<NeoId> = ["V9H0G6C4", "C5U6K1E7"]
            .iter()
            .filter_map(|s| NeoId::parse(s))
            .collect();
        let b: BTreeSet<NeoId> = ["C5U6K1E7", "V9H0G6C4"]
            .iter()
            .filter_map(|s| NeoId::parse(s))
            .collect();
        assert_eq!(
            hash_content(&a, &BTreeSet::new()),
            hash_content(&b, &BTreeSet::new())
        );
    }

    #[test]
    fn content_hash_separates_different_sets() {
        let a: BTreeSet<NeoId> = ["V9H0G6C4"]
            .iter()
            .filter_map(|s| NeoId::parse(s))
            .collect();
        let b: BTreeSet<NeoId> = ["C5U6K1E7"]
            .iter()
            .filter_map(|s| NeoId::parse(s))
            .collect();
        assert_ne!(
            hash_content(&a, &BTreeSet::new()),
            hash_content(&b, &BTreeSet::new())
        );
    }
}
