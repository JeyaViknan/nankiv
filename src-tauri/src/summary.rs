//! The summary a student pastes into the group chat.
//!
//! Set for WhatsApp, where a batch talks: the company in *bold*, the hedge in
//! _italics_, one fact to a line. It says what the group wants to know (which
//! list, how many made it, what it took) and nothing it would have to decode:
//! no sample sizes, no sign-off, and never the student's own status, which is
//! theirs to say.

use crate::analytics::cutoff::CutoffVerdict;
use crate::analytics::DriveAnalysis;
use crate::engine;
use crate::store::DriveRecord;

/// The summary for `drive`, given every stored drive (for its earlier round).
pub fn group_chat(drive: &DriveRecord, drives: &[DriveRecord], analysis: &DriveAnalysis) -> String {
    let rounds = engine::round_labels(drives);
    let label = |d: &DriveRecord| rounds.get(&d.id).cloned().or_else(|| d.round_label.clone());

    // Asterisks in a name would end the bold early.
    let mut title = format!("*{}*", drive.company.replace('*', "").trim());
    if let Some(r) = label(drive) {
        title.push_str(&format!(" · {r}"));
    }
    let mut count = format!("{} shortlisted", thousands(drive.total_students));
    if let Some(d) = &drive.drive_date {
        count.push_str(&format!(" · {}", day(d)));
    }
    let mut lines = vec![title, count];

    if let Some(before) = drive
        .parent_drive_id
        .and_then(|id| drives.iter().find(|d| d.id == id))
    {
        let n = thousands(before.total_students);
        lines.push(match label(before) {
            Some(r) => format!("From {n} in {r}"),
            None => format!("From {n} in the round before"),
        });
    }

    // What it took, only when there was enough to say it.
    let mut findings = Vec::new();
    if analysis.sufficient {
        if let Some(c) = &analysis.cutoff {
            findings.push(match c.verdict.value {
                CutoffVerdict::HardCutoff { threshold, .. } => {
                    format!("CGPA cutoff ≈ {threshold:.1} _(estimate)_")
                }
                CutoffVerdict::SoftPreference { .. } => {
                    "CGPA skews high, no hard cutoff _(estimate)_".to_string()
                }
                CutoffVerdict::NoCgpaFilter => "No CGPA cutoff seen".to_string(),
            });
        }
        if let Some(b) = &analysis.branches {
            if !b.over_represented.is_empty() {
                findings.push(format!(
                    "More {} than the batch",
                    b.over_represented.join(" and ")
                ));
            }
        }
    }
    if !findings.is_empty() {
        lines.push(String::new());
        lines.extend(findings);
    }
    lines.join("\n")
}

/// 1204 → "1,204".
fn thousands(n: usize) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// A file's date as one form, "16 Sep", whichever way it was written:
/// 16-09-26, 28/09/2026, 2026-09-28. Anything else is left as the file had it.
fn day(raw: &str) -> String {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let parts: Vec<&str> = raw.trim().split(['-', '/', '.']).collect();
    let num = |s: &str| -> Option<u32> {
        (!s.is_empty() && s.chars().all(|c| c.is_ascii_digit()))
            .then(|| s.parse().ok())
            .flatten()
    };
    let (d, m) = match parts.as_slice() {
        [y, m, d] if y.len() == 4 && num(y).is_some() => (num(d), num(m)),
        [d, m, y] if num(y).is_some() => (num(d), num(m)),
        _ => (None, None),
    };
    match (d, m) {
        (Some(d), Some(m)) if (1..=31).contains(&d) && (1..=12).contains(&m) => {
            format!("{d} {}", MONTHS[m as usize - 1])
        }
        _ => raw.trim().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::branch::BranchReport;
    use crate::analytics::cutoff::CutoffReport;
    use crate::model::Estimate;

    fn drive(id: i64, company: &str, total: usize) -> DriveRecord {
        DriveRecord {
            id,
            company: company.into(),
            drive_date: None,
            imported_at: "2026-10-05 09:00:00".into(),
            source_filename: format!("{company}.xlsx"),
            content_hash: format!("h{id}"),
            shape: "neo_id".into(),
            primary_key: Some("neo_id".into()),
            total_students: total,
            round_label: None,
            parent_drive_id: None,
        }
    }

    fn analysis(cutoff: Option<CutoffVerdict>, over: &[&str], sufficient: bool) -> DriveAnalysis {
        DriveAnalysis {
            total_students: 767,
            matched_students: 300,
            coverage: 0.39,
            sufficient,
            cgpa: None,
            cutoff: cutoff.map(|v| CutoffReport {
                verdict: Estimate::new(v, 300, 767),
                comparison: Vec::new(),
                statement: String::new(),
            }),
            branches: Some(BranchReport {
                rows: Estimate::new(Vec::new(), 300, 767),
                over_represented: over.iter().map(|s| s.to_string()).collect(),
                statement: String::new(),
            }),
            your_percentile: None,
        }
    }

    #[test]
    fn a_later_round_says_where_it_came_from_and_what_it_took() {
        // The first round's file never named itself; the second called itself
        // an interview.
        let test = drive(1, "Axxela", 1204);
        let mut interview = drive(2, "Axxela", 767);
        interview.round_label = Some("Interview".into());
        interview.parent_drive_id = Some(1);
        interview.drive_date = Some("05-10-26".into());
        let drives = vec![interview.clone(), test];

        let cutoff = CutoffVerdict::HardCutoff {
            threshold: 8.0,
            observed_floor: 8.03,
        };
        assert_eq!(
            group_chat(&interview, &drives, &analysis(Some(cutoff), &["ECE"], true)),
            "*Axxela* · Interview\n\
             767 shortlisted · 5 Oct\n\
             From 1,204 in R1\n\
             \n\
             CGPA cutoff ≈ 8.0 _(estimate)_\n\
             More ECE than the batch"
        );
    }

    #[test]
    fn a_single_list_says_only_what_there_is() {
        let d = drive(1, "Citi", 74);
        // Too few matched to say anything: no findings, and no blank line.
        let thin = analysis(Some(CutoffVerdict::NoCgpaFilter), &["ECE"], false);
        assert_eq!(
            group_chat(&d, std::slice::from_ref(&d), &thin),
            "*Citi*\n74 shortlisted"
        );

        // A branch mix like the batch's is not news.
        let plain = analysis(Some(CutoffVerdict::NoCgpaFilter), &[], true);
        assert_eq!(
            group_chat(&d, std::slice::from_ref(&d), &plain),
            "*Citi*\n74 shortlisted\n\nNo CGPA cutoff seen"
        );
    }

    #[test]
    fn every_finding_about_cgpa_is_hedged() {
        let d = drive(1, "Citi", 74);
        for v in [
            CutoffVerdict::HardCutoff {
                threshold: 8.5,
                observed_floor: 8.51,
            },
            CutoffVerdict::SoftPreference {
                observed_floor: 7.9,
            },
            CutoffVerdict::NoCgpaFilter,
        ] {
            let text = group_chat(&d, std::slice::from_ref(&d), &analysis(Some(v), &[], true));
            let line = text.lines().last().unwrap().to_lowercase();
            assert!(!line.contains("official"), "{line}");
            assert!(line.contains("estimate") || line.contains("seen"), "{line}");
        }
    }

    #[test]
    fn it_never_says_how_you_did() {
        let d = drive(1, "Citi", 74);
        let text =
            group_chat(&d, std::slice::from_ref(&d), &analysis(None, &[], true)).to_lowercase();
        for word in ["you", "i'm", "not in", "shortlisted you"] {
            assert!(!text.contains(word), "{word} in {text}");
        }
    }

    #[test]
    fn dates_read_one_way() {
        assert_eq!(day("16-09-26"), "16 Sep");
        assert_eq!(day("28-09-2026"), "28 Sep");
        assert_eq!(day("28/09/2026"), "28 Sep");
        assert_eq!(day("2026-09-28"), "28 Sep");
        assert_eq!(day("Sept 28"), "Sept 28");
        assert_eq!(day("31-13-26"), "31-13-26");
    }

    #[test]
    fn counts_are_grouped() {
        assert_eq!(thousands(74), "74");
        assert_eq!(thousands(1204), "1,204");
        assert_eq!(thousands(1_204_000), "1,204,000");
    }
}
