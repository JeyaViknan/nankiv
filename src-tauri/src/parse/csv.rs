//! Minimal CSV reading.
//!
//! The application advertises `.csv` in its drag overlay and its file dialog,
//! but the spreadsheet backend cannot read one — a dropped CSV failed with
//! "Cannot detect file format", which is a promise the product was not keeping.
//! A student exporting from Google Sheets gets a CSV by default, so the promise
//! is worth keeping rather than withdrawing.
//!
//! Deliberately small: this reads a grid of strings for identifier detection.
//! It handles quoting and embedded separators because real exports contain
//! names like `Gupta, Shresth`, and it does not attempt to type-infer, because
//! everything downstream works from text anyway.

use calamine::Data;

/// Splits CSV text into a grid, honouring RFC 4180 quoting.
pub fn parse_grid(text: &str) -> Vec<Vec<Data>> {
    let delimiter = detect_delimiter(text);
    let mut grid = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                // A doubled quote inside a quoted field is a literal quote.
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    quoted = false;
                }
            } else {
                field.push(c);
            }
        } else if c == '"' && field.is_empty() {
            quoted = true;
        } else if c == delimiter {
            row.push(Data::String(field.trim().to_string()));
            field = String::new();
        } else if c == '\n' {
            row.push(Data::String(field.trim().to_string()));
            grid.push(std::mem::take(&mut row));
            field = String::new();
        } else if c != '\r' {
            field.push(c);
        }
    }

    if !field.is_empty() || !row.is_empty() {
        row.push(Data::String(field.trim().to_string()));
        grid.push(row);
    }

    grid
}

/// Picks the separator by counting candidates in the first few lines.
///
/// Semicolon-separated exports are common wherever the comma is a decimal
/// separator, and tab-separated ones come from pasting out of a spreadsheet.
fn detect_delimiter(text: &str) -> char {
    let sample: String = text.lines().take(5).collect::<Vec<_>>().join("\n");
    [',', ';', '\t']
        .into_iter()
        .max_by_key(|d| sample.matches(*d).count())
        .filter(|d| sample.contains(*d))
        .unwrap_or(',')
}

pub fn is_csv(path: &std::path::Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .as_deref(),
        Some("csv" | "tsv" | "txt")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::cell_text;

    fn cells(grid: &[Vec<Data>]) -> Vec<Vec<String>> {
        grid.iter()
            .map(|r| r.iter().map(cell_text).collect())
            .collect()
    }

    #[test]
    fn reads_a_plain_shortlist() {
        let g = parse_grid("Neo ID\nV9H0G6C4\nC5U6K1E7\n");
        assert_eq!(
            cells(&g),
            vec![vec!["Neo ID"], vec!["V9H0G6C4"], vec!["C5U6K1E7"]]
        );
    }

    #[test]
    fn honours_quoted_fields_containing_the_separator() {
        // Real exports carry names like `Gupta, Shresth`.
        let g = parse_grid("Neo ID,Name\nV9H0G6C4,\"Gupta, Shresth\"\n");
        assert_eq!(cells(&g)[1], vec!["V9H0G6C4", "Gupta, Shresth"]);
    }

    #[test]
    fn handles_doubled_quotes() {
        let g = parse_grid("Name\n\"She said \"\"hi\"\"\"\n");
        assert_eq!(cells(&g)[1], vec![r#"She said "hi""#]);
    }

    #[test]
    fn survives_windows_line_endings() {
        let g = parse_grid("Neo ID\r\nV9H0G6C4\r\n");
        assert_eq!(cells(&g)[1], vec!["V9H0G6C4"]);
    }

    #[test]
    fn keeps_a_final_row_without_a_trailing_newline() {
        let g = parse_grid("Neo ID\nV9H0G6C4");
        assert_eq!(g.len(), 2);
    }

    #[test]
    fn detects_semicolon_and_tab_separated_exports() {
        assert_eq!(
            cells(&parse_grid("Neo ID;Name\nV9H0G6C4;Monish\n"))[1],
            vec!["V9H0G6C4", "Monish"]
        );
        assert_eq!(
            cells(&parse_grid("Neo ID\tName\nV9H0G6C4\tMonish\n"))[1],
            vec!["V9H0G6C4", "Monish"]
        );
    }

    #[test]
    fn an_empty_file_yields_an_empty_grid() {
        assert!(parse_grid("").is_empty());
    }

    #[test]
    fn recognises_the_extensions_the_interface_advertises() {
        use std::path::Path;
        assert!(is_csv(Path::new("a.csv")));
        assert!(is_csv(Path::new("a.CSV")));
        assert!(is_csv(Path::new("a.tsv")));
        assert!(!is_csv(Path::new("a.xlsx")));
    }
}
