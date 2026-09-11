//! Column election and file-shape classification.
//!
//! Columns are elected by counting how many cells in each column match a known
//! identifier shape. The column with the highest density wins for that
//! identifier type. Header text is read only so that an unreadable file can
//! quote back what it found.

use super::{cell_text, ParsedRow};
use crate::model::{NeoId, RegNo};
use calamine::Data;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// A column must be at least this proportion identifiers to be elected. Guards
/// against a stray ID appearing in a notes or remarks column.
const MIN_DENSITY: f64 = 0.5;
/// And must contain at least this many, so a two-row file cannot elect a column
/// off a single lucky cell.
const MIN_HITS: usize = 2;

/// The four shapes observed across the real corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileShape {
    /// Neo ID only — ten of fifteen sampled files.
    NeoIdOnly,
    /// Registration number only — Deloitte, HPE.
    RegNoOnly,
    /// Both keys in the same row. The most valuable shape: it donates verified
    /// identity links for every row.
    Linked,
    /// No recognised identifier. The TCS/Cognizant `REFERENCE_ID` case.
    Unrecognised,
}

impl FileShape {
    pub fn classify(neo: &BTreeSet<NeoId>, reg: &BTreeSet<RegNo>, rows: &[ParsedRow]) -> FileShape {
        let linked = rows
            .iter()
            .filter(|r| r.neo_id.is_some() && r.reg_no.is_some())
            .count();
        if linked >= MIN_HITS {
            FileShape::Linked
        } else if !neo.is_empty() {
            FileShape::NeoIdOnly
        } else if !reg.is_empty() {
            FileShape::RegNoOnly
        } else {
            FileShape::Unrecognised
        }
    }

    pub fn is_usable(self) -> bool {
        !matches!(self, FileShape::Unrecognised)
    }
}

/// Which column index holds which identifier, for one sheet.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ColumnLayout {
    pub neo_col: Option<usize>,
    pub reg_col: Option<usize>,
    pub name_col: Option<usize>,
}

impl ColumnLayout {
    /// True when no identifier column could be elected, so the sheet carries
    /// nothing we can use.
    pub fn is_barren(&self) -> bool {
        self.neo_col.is_none() && self.reg_col.is_none()
    }

    /// Pulls the elected fields out of one row.
    pub fn extract(&self, row: &[Data]) -> ParsedRow {
        let at = |idx: Option<usize>| -> Option<String> {
            idx.and_then(|i| row.get(i))
                .map(cell_text)
                .filter(|s| !s.is_empty())
        };

        ParsedRow {
            neo_id: at(self.neo_col).and_then(|s| NeoId::parse(&s)),
            reg_no: at(self.reg_col).and_then(|s| RegNo::parse(&s)),
            name: at(self.name_col).filter(|s| looks_like_name(s)),
        }
    }

    /// Header text sitting above the elected columns, if the first row is textual.
    pub fn header_labels(&self, grid: &[Vec<Data>]) -> Vec<String> {
        let Some(first) = grid.first() else {
            return Vec::new();
        };
        [self.neo_col, self.reg_col, self.name_col]
            .iter()
            .filter_map(|c| c.and_then(|i| first.get(i)))
            .map(cell_text)
            .filter(|s| !s.is_empty() && NeoId::parse(s).is_none() && RegNo::parse(s).is_none())
            .collect()
    }
}

/// Elects identifier columns by shape density.
pub fn elect_columns(grid: &[Vec<Data>]) -> ColumnLayout {
    let width = grid.iter().map(|r| r.len()).max().unwrap_or(0);
    if width == 0 {
        return ColumnLayout::default();
    }

    let mut neo_hits = vec![0usize; width];
    let mut reg_hits = vec![0usize; width];
    let mut name_hits = vec![0usize; width];
    let mut filled = vec![0usize; width];

    for row in grid {
        for (i, cell) in row.iter().enumerate() {
            let t = cell_text(cell);
            if t.is_empty() {
                continue;
            }
            filled[i] += 1;
            if NeoId::parse(&t).is_some() {
                neo_hits[i] += 1;
            } else if RegNo::parse(&t).is_some() {
                reg_hits[i] += 1;
            } else if looks_like_name(&t) {
                name_hits[i] += 1;
            }
        }
    }

    let best = |hits: &[usize]| -> Option<usize> {
        hits.iter()
            .enumerate()
            .filter(|(i, &h)| {
                h >= MIN_HITS && filled[*i] > 0 && (h as f64 / filled[*i] as f64) >= MIN_DENSITY
            })
            .max_by_key(|(_, &h)| h)
            .map(|(i, _)| i)
    };

    let neo_col = best(&neo_hits);
    let reg_col = best(&reg_hits);

    // A name column is only meaningful next to an identifier, and must not be a
    // column we already elected.
    let name_col = if neo_col.is_some() || reg_col.is_some() {
        name_hits
            .iter()
            .enumerate()
            .filter(|(i, &h)| {
                Some(*i) != neo_col
                    && Some(*i) != reg_col
                    && h >= MIN_HITS
                    && filled[*i] > 0
                    && (h as f64 / filled[*i] as f64) >= MIN_DENSITY
            })
            .max_by_key(|(_, &h)| h)
            .map(|(i, _)| i)
    } else {
        None
    };

    ColumnLayout {
        neo_col,
        reg_col,
        name_col,
    }
}

/// A loose test for "this cell holds a person's name".
///
/// Intentionally conservative: names are only ever a hint, and a false positive
/// here would feed noise into the identity graph.
pub fn looks_like_name(s: &str) -> bool {
    let t = s.trim();
    if t.len() < 3 || t.len() > 60 {
        return false;
    }
    if t.contains('@') || t.contains("://") {
        return false;
    }
    let letters = t.chars().filter(|c| c.is_alphabetic()).count();
    let digits = t.chars().filter(|c| c.is_ascii_digit()).count();
    if digits > 0 || letters < 3 {
        return false;
    }
    // Reject obvious header words.
    let lower = t.to_ascii_lowercase();
    const HEADERS: [&str; 12] = [
        "name",
        "full name",
        "candidate name",
        "student name",
        "venue",
        "remarks",
        "role",
        "batch",
        "gender",
        "degree",
        "branch",
        "campus",
    ];
    if HEADERS.contains(&lower.as_str()) {
        return false;
    }
    letters as f64 / t.chars().count() as f64 > 0.7
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Data {
        Data::String(v.to_string())
    }

    #[test]
    fn elects_the_single_neo_id_column() {
        // The Elgi / Fractal / Amazon shape: one column, one header.
        let grid = vec![
            vec![s("Neo ID")],
            vec![s("E4P3P9H4")],
            vec![s("G2Z6V7H4")],
            vec![s("S3N3Q3P8")],
        ];
        let l = elect_columns(&grid);
        assert_eq!(l.neo_col, Some(0));
        assert_eq!(l.reg_col, None);
    }

    #[test]
    fn ignores_serial_and_batch_columns() {
        // The AU Small shape: Sno, Batch, Neo ID.
        let grid = vec![
            vec![s("Sno"), s("Batch"), s("Neo ID")],
            vec![s("1"), s("Batch 1 "), s("Z5A2H0Z9")],
            vec![s("2"), s(""), s("T2U2J3P8")],
            vec![s("3"), s(""), s("T6Q3O8A6")],
        ];
        let l = elect_columns(&grid);
        assert_eq!(l.neo_col, Some(2));
    }

    #[test]
    fn finds_both_keys_in_the_linked_shape() {
        // The Tredence shape — the file that donates verified identity links.
        let grid = vec![
            vec![
                s("S.No"),
                s("NEO ID"),
                s("Register Number"),
                s("Name"),
                s("Venue"),
            ],
            vec![
                s("1"),
                s("E2S8L9L8"),
                s("23BCE1473"),
                s("Monish D"),
                s("AB2-501"),
            ],
            vec![
                s("2"),
                s("X2K9T4U5"),
                s("23BCE1633"),
                s("Dhruv Sahni"),
                s("AB2-501"),
            ],
            vec![
                s("3"),
                s("R2R1R5B7"),
                s("23BCE1481"),
                s("Yuvaraj U"),
                s("AB2-501"),
            ],
        ];
        let l = elect_columns(&grid);
        assert_eq!(l.neo_col, Some(1));
        assert_eq!(l.reg_col, Some(2));
        assert_eq!(l.name_col, Some(3));
    }

    #[test]
    fn handles_reg_no_keyed_files() {
        // The HPE shape.
        let grid = vec![
            vec![s("Reg.no"), s("Candidate Name")],
            vec![s("23BAI0001"), s("Agnik Patra")],
            vec![s("23BAI0011"), s("Yash Agarwal")],
            vec![s("23BAI0036"), s("Aditya Rajeev Nair")],
        ];
        let l = elect_columns(&grid);
        assert_eq!(l.reg_col, Some(0));
        assert_eq!(l.neo_col, None);
        assert_eq!(l.name_col, Some(1));
    }

    #[test]
    fn reports_barren_for_foreign_identifiers() {
        // The TCS/Cognizant shape. Must not elect anything.
        let grid = vec![
            vec![s("REFERENCE_ID"), s("Interview Date")],
            vec![s("CT20264996884"), s("27th Aug")],
            vec![s("CT20265000047"), s("27th Aug")],
            vec![s("DT20268151988"), s("27th Aug")],
        ];
        let l = elect_columns(&grid);
        assert!(l.is_barren(), "TCS reference ids must not be elected");
    }

    #[test]
    fn a_stray_id_in_a_notes_column_is_not_elected() {
        let grid = vec![
            vec![s("Neo ID"), s("Remarks")],
            vec![s("E4P3P9H4"), s("duplicate of V9H0G6C4")],
            vec![s("G2Z6V7H4"), s("ok")],
            vec![s("S3N3Q3P8"), s("ok")],
            vec![s("Q4D5H3A2"), s("ok")],
        ];
        let l = elect_columns(&grid);
        assert_eq!(l.neo_col, Some(0));
    }

    #[test]
    fn empty_grid_is_barren() {
        assert!(elect_columns(&[]).is_barren());
        assert!(elect_columns(&[vec![]]).is_barren());
    }

    #[test]
    fn name_detection_rejects_non_names() {
        assert!(looks_like_name("Monish D"));
        assert!(looks_like_name("Aditya Rajeev Nair"));
        assert!(!looks_like_name("23BCE1473"));
        assert!(!looks_like_name("a@b.com"));
        assert!(!looks_like_name("Name"), "header word");
        assert!(!looks_like_name("AB2-501"), "venue code");
        assert!(!looks_like_name("x"));
    }

    #[test]
    fn shape_classification_matches_the_corpus() {
        let neo: BTreeSet<NeoId> = ["V9H0G6C4"]
            .iter()
            .filter_map(|s| NeoId::parse(s))
            .collect();
        let reg: BTreeSet<RegNo> = ["23BAI0001"]
            .iter()
            .filter_map(|s| RegNo::parse(s))
            .collect();
        let empty_n = BTreeSet::new();
        let empty_r = BTreeSet::new();

        assert_eq!(
            FileShape::classify(&neo, &empty_r, &[]),
            FileShape::NeoIdOnly
        );
        assert_eq!(
            FileShape::classify(&empty_n, &reg, &[]),
            FileShape::RegNoOnly
        );
        assert_eq!(
            FileShape::classify(&empty_n, &empty_r, &[]),
            FileShape::Unrecognised
        );
        assert!(!FileShape::Unrecognised.is_usable());
    }
}
