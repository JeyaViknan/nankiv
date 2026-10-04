//! A shortlist pasted as text.
//!
//! Shortlists do not always arrive as spreadsheets. Some are a column of Neo
//! IDs in an email, some a WhatsApp forward with names, numbers and greetings
//! mixed in. This reads whatever was pasted and keeps exactly one thing: the
//! identifiers. Every other word on every line is ignored and never stored.
//!
//! Each identifier is its own row. A file's row puts a Neo ID beside the
//! registration number of the same student; a line of chat makes no such
//! promise — "V9H0G6C4, C5U6K1E7" is two people — so pasted text never links
//! one identifier to another, and never attaches a name. Lines of words with
//! no identifier are kept only as hints for naming the drive.

use super::{hash_content, FileShape, ParseError, ParsedFile, ParsedRow, RowOrigin};
use crate::model::{KeyKind, NeoId, RegNo};
use std::collections::BTreeSet;

/// The sheet name a pasted list is recorded under, and its source filename.
pub const PASTED_SHEET: &str = "Pasted list";

/// A long message, not a database dump.
pub const MAX_TEXT_BYTES: usize = 1024 * 1024;
const MAX_TITLES: usize = 3;
const MAX_TITLE_CHARS: usize = 120;

/// What came out of a paste.
#[derive(Debug, Clone)]
pub struct PastedText {
    pub parsed: ParsedFile,
    /// Lines with words on them but no identifier, which were skipped.
    pub unread_lines: usize,
}

pub fn parse_text(text: &str) -> Result<PastedText, ParseError> {
    if text.len() > MAX_TEXT_BYTES {
        return Err(ParseError::TooLarge(text.len() as u64));
    }

    let mut rows = Vec::new();
    let mut origins = Vec::new();
    let mut titles = Vec::new();
    let mut unread_lines = 0;

    for (i, line) in text.lines().enumerate() {
        let mut found = false;
        for token in line.split(|c: char| !c.is_ascii_alphanumeric()) {
            let row = if let Some(n) = NeoId::parse(token) {
                ParsedRow {
                    neo_id: Some(n),
                    reg_no: None,
                    name: None,
                }
            } else if let Some(r) = RegNo::parse(token) {
                ParsedRow {
                    neo_id: None,
                    reg_no: Some(r),
                    name: None,
                }
            } else {
                continue;
            };
            found = true;
            rows.push(row);
            origins.push(RowOrigin {
                sheet: PASTED_SHEET.to_string(),
                row: i as u32 + 1,
                neo_column: None,
                reg_column: None,
            });
        }

        let words = line.chars().filter(|c| c.is_alphabetic()).count();
        if !found && words >= 3 {
            unread_lines += 1;
            if titles.len() < MAX_TITLES {
                titles.push(line.trim().chars().take(MAX_TITLE_CHARS).collect());
            }
        }
    }

    if rows.is_empty() {
        return Err(ParseError::Empty);
    }

    let neo_ids: BTreeSet<NeoId> = rows.iter().filter_map(|r| r.neo_id.clone()).collect();
    let reg_nos: BTreeSet<RegNo> = rows.iter().filter_map(|r| r.reg_no.clone()).collect();
    let primary_key = if neo_ids.len() >= reg_nos.len() {
        Some(KeyKind::NeoId)
    } else {
        Some(KeyKind::RegNo)
    };
    let shape = FileShape::classify(&neo_ids, &reg_nos, &rows);
    let content_hash = hash_content(&neo_ids, &reg_nos);

    Ok(PastedText {
        parsed: ParsedFile {
            rows,
            shape,
            neo_ids,
            reg_nos,
            primary_key,
            content_hash,
            observed_headers: vec![],
            sheet_names: vec![PASTED_SHEET.to_string()],
            origins,
            titles,
        },
        unread_lines,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHAT: &str = "\
Hi all, Siemens SISW shortlist for the interview round:
1. Arjun R - V9H0G6C4
2. Meera K - c5u6k1e7
3) Ravi, E2S8L9L8, 23BCE1473
Reporting time 9:30 AM at SJT 3rd floor
V9H0G6C4 (repeated)
";

    #[test]
    fn only_identifiers_are_taken_from_a_chat_message() {
        let p = parse_text(CHAT).unwrap();
        let neo: Vec<&str> = p.parsed.neo_ids.iter().map(|n| n.as_str()).collect();
        assert_eq!(
            neo,
            ["C5U6K1E7", "E2S8L9L8", "V9H0G6C4"],
            "case-insensitive, deduplicated"
        );
        let reg: Vec<&str> = p.parsed.reg_nos.iter().map(|r| r.as_str()).collect();
        assert_eq!(reg, ["23BCE1473"]);
        assert_eq!(p.parsed.primary_key, Some(KeyKind::NeoId));
        assert!(p.parsed.shape.is_usable());
    }

    #[test]
    fn nothing_but_identifiers_is_kept() {
        let p = parse_text(CHAT).unwrap();
        // No names, and no link between the two identifiers on Ravi's line:
        // a line of chat is not a row of a spreadsheet.
        assert!(p.parsed.rows.iter().all(|r| r.name.is_none()));
        assert!(p
            .parsed
            .rows
            .iter()
            .all(|r| r.neo_id.is_none() || r.reg_no.is_none()));
    }

    #[test]
    fn each_identifier_knows_its_line() {
        let p = parse_text(CHAT).unwrap();
        let line_of = |id: &str| {
            let i = p
                .parsed
                .rows
                .iter()
                .position(|r| r.neo_id.as_ref().map(|n| n.as_str()) == Some(id))
                .unwrap();
            p.parsed.origins[i].clone()
        };
        let meera = line_of("C5U6K1E7");
        assert_eq!((meera.sheet.as_str(), meera.row), (PASTED_SHEET, 3));
        assert_eq!(meera.neo_column, None);
    }

    #[test]
    fn the_words_around_them_only_help_name_the_drive() {
        let p = parse_text(CHAT).unwrap();
        assert_eq!(p.unread_lines, 2);
        assert_eq!(
            p.parsed.titles[0],
            "Hi all, Siemens SISW shortlist for the interview round:"
        );
    }

    #[test]
    fn the_same_list_pasted_twice_is_recognised() {
        let a = parse_text("V9H0G6C4\nC5U6K1E7").unwrap();
        let b = parse_text("c5u6k1e7, v9h0g6c4 — thanks!").unwrap();
        assert_eq!(a.parsed.content_hash, b.parsed.content_hash);
    }

    #[test]
    fn text_with_no_identifiers_is_refused() {
        assert!(matches!(
            parse_text("See you all at 9"),
            Err(ParseError::Empty)
        ));
        assert!(matches!(parse_text(""), Err(ParseError::Empty)));
    }

    #[test]
    fn a_dump_is_refused() {
        let huge = "V9H0G6C4\n".repeat(MAX_TEXT_BYTES / 8);
        assert!(matches!(parse_text(&huge), Err(ParseError::TooLarge(_))));
    }
}
