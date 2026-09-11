//! Branch analysis.
//!
//! Branch is categorical, so it stays meaningful at sample sizes where CGPA does
//! not — and companies filter on it heavily. In the sample corpus the Siemens
//! shortlist was 32% ECE against a batch that is roughly 7.5% ECE: a four-fold
//! over-representation, and a far more actionable finding than any CGPA number
//! from that file.
//!
//! Canonicalisation matters: the reference sheet holds 507 distinct branch
//! spellings that collapse to roughly fifteen real branches.

use super::Baseline;
use crate::model::{sample_is_sufficient, Estimate};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Collapses a free-text branch string to a canonical label.
///
/// Order matters: the more specific specialisations are tested before the
/// generic `CSE`, or `CSE (AI & ML)` would be swallowed by it.
pub fn canonicalise(raw: &str) -> String {
    let s = raw.to_ascii_lowercase();
    let has = |pat: &str| s.contains(pat);

    // Specialisations first.
    if has("ai") && (has("ml") || has("machine")) || has("artificial intelligence") {
        return "CSE (AI & ML)".into();
    }
    if has("data sci") || has("big data") || has("data analytics") {
        return "CSE (Data Science)".into();
    }
    if has("cyber") || has("information security") || has("block chain") || has("blockchain") {
        return "CSE (Cyber Security)".into();
    }
    if has("internet of things") || has("iot") {
        return "CSE (IoT)".into();
    }
    if has("business system") {
        return "CSE (Business Systems)".into();
    }
    // Then the broad branches.
    if has("information tech") || s.trim() == "it" {
        return "IT".into();
    }
    if has("electronics") && has("comm") || s.trim() == "ece" {
        return "ECE".into();
    }
    if has("electronics") && (has("computer") || has("vlsi")) {
        return "ECM".into();
    }
    if has("electrical") || s.trim() == "eee" {
        return "EEE".into();
    }
    if has("mech") {
        return "MECH".into();
    }
    if has("civil") {
        return "CIVIL".into();
    }
    if has("chemical") {
        return "CHEM".into();
    }
    if has("bio") {
        return "BIO".into();
    }
    if has("cse") || has("computer sci") || has("computer engineering") {
        return "CSE".into();
    }
    if s.trim().is_empty() {
        return "Unknown".into();
    }
    "Other".into()
}

/// One branch's representation in a shortlist, against the batch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BranchLift {
    pub branch: String,
    pub count: usize,
    pub share: f64,
    /// Share of the batch in this branch. `None` when no branch baseline exists.
    pub baseline_share: Option<f64>,
    /// share / baseline_share. Above 1.0 means over-represented.
    pub lift: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BranchReport {
    pub rows: Estimate<Vec<BranchLift>>,
    /// Branches at least twice as common here as in the batch, strongest first.
    pub over_represented: Vec<String>,
    pub statement: String,
}

/// Builds the branch report, or `None` when the sample cannot support it.
pub fn analyse(
    branches: &[String],
    baseline: &Baseline,
    matched: usize,
    total: usize,
) -> Option<BranchReport> {
    if branches.is_empty() || !sample_is_sufficient(matched, total) {
        return None;
    }

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for b in branches {
        *counts.entry(canonicalise(b)).or_insert(0) += 1;
    }
    let n = branches.len() as f64;

    let mut rows: Vec<BranchLift> = counts
        .into_iter()
        .map(|(branch, count)| {
            let share = count as f64 / n;
            let baseline_share = if baseline.has_branches() {
                Some(baseline.branch_share(&branch))
            } else {
                None
            };
            let lift = baseline_share.and_then(|b| if b > 0.0 { Some(share / b) } else { None });
            BranchLift {
                branch,
                count,
                share,
                baseline_share,
                lift,
            }
        })
        .collect();

    rows.sort_by(|a, b| b.count.cmp(&a.count).then(a.branch.cmp(&b.branch)));

    let mut over: Vec<(String, f64)> = rows
        .iter()
        .filter(|r| r.lift.map(|l| l >= 2.0).unwrap_or(false) && r.count >= 3)
        .map(|r| (r.branch.clone(), r.lift.unwrap_or(0.0)))
        .collect();
    over.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    let over_represented: Vec<String> = over.into_iter().map(|(b, _)| b).collect();

    let statement = if over_represented.is_empty() {
        let top = rows.first().map(|r| r.branch.as_str()).unwrap_or("unknown");
        format!(
            "Branch mix looks broadly like the batch, led by {top} (based on {matched} of {total} students)."
        )
    } else {
        format!(
            "{} {} noticeably over-represented compared to the batch (based on {matched} of {total} students).",
            over_represented.join(" and "),
            if over_represented.len() == 1 { "is" } else { "are" }
        )
    };

    Some(BranchReport {
        rows: Estimate::new(rows, matched, total),
        over_represented,
        statement,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalisation_collapses_real_spellings() {
        // Every one of these appears in the reference sheet.
        for s in [
            "CSE (AIML)",
            "CSE (AI and ML)",
            "Cse (AI ML)",
            "Cse(ai&ml)",
            "Cse Ai Ml",
            "CSE with AI & ML",
            "CSE- Artificial Intelligence and Machine Learning",
            "CSE with AI/ML",
        ] {
            assert_eq!(canonicalise(s), "CSE (AI & ML)", "failed for {s}");
        }
    }

    #[test]
    fn plain_cse_variants_collapse_together() {
        for s in [
            "CSE",
            "cse core",
            "CSE Core",
            "Computer Science and Engineering",
            "computer science",
            "Computer Science Engineering",
        ] {
            assert_eq!(canonicalise(s), "CSE", "failed for {s}");
        }
    }

    #[test]
    fn specialisations_are_not_swallowed_by_generic_cse() {
        assert_eq!(canonicalise("CSE (Data Science)"), "CSE (Data Science)");
        assert_eq!(
            canonicalise("CSE with cyber security"),
            "CSE (Cyber Security)"
        );
        assert_ne!(canonicalise("CSE with AI & ML"), "CSE");
    }

    #[test]
    fn other_branches_map_correctly() {
        assert_eq!(canonicalise("ECE"), "ECE");
        assert_eq!(canonicalise("Electronics and Communication"), "ECE");
        assert_eq!(canonicalise("information technology"), "IT");
        assert_eq!(canonicalise("Mechanical Engineering"), "MECH");
        assert_eq!(canonicalise("civil"), "CIVIL");
        assert_eq!(canonicalise(""), "Unknown");
    }

    #[test]
    fn insufficient_samples_produce_no_report() {
        let b = Baseline::default();
        assert!(analyse(&["CSE".into()], &b, 1, 125).is_none());
        assert!(analyse(&[], &b, 50, 100).is_none());
    }

    #[test]
    fn the_siemens_ece_signal_is_detected() {
        // 32% ECE in the shortlist against ~7.5% in the batch is a 4x lift.
        let baseline = Baseline::from_values(&[8.0; 100]).with_branches(
            &(0..1000)
                .map(|i| {
                    if i < 75 {
                        "ECE".to_string()
                    } else {
                        "CSE".to_string()
                    }
                })
                .collect::<Vec<_>>(),
        );
        let shortlist: Vec<String> = (0..25)
            .map(|i| {
                if i < 8 {
                    "ECE".to_string()
                } else {
                    "CSE".to_string()
                }
            })
            .collect();

        let r = analyse(&shortlist, &baseline, 25, 166).expect("report");
        assert!(
            r.over_represented.contains(&"ECE".to_string()),
            "ECE should be flagged, got {:?}",
            r.over_represented
        );
        let ece = r.rows.value.iter().find(|x| x.branch == "ECE").unwrap();
        assert!(ece.lift.unwrap() > 3.0, "lift was {:?}", ece.lift);
    }

    #[test]
    fn a_representative_mix_is_not_flagged() {
        let baseline = Baseline::from_values(&[8.0; 100]).with_branches(
            &(0..1000)
                .map(|i| {
                    if i < 750 {
                        "CSE".to_string()
                    } else {
                        "ECE".to_string()
                    }
                })
                .collect::<Vec<_>>(),
        );
        let shortlist: Vec<String> = (0..40)
            .map(|i| {
                if i < 30 {
                    "CSE".to_string()
                } else {
                    "ECE".to_string()
                }
            })
            .collect();
        let r = analyse(&shortlist, &baseline, 40, 100).expect("report");
        assert!(
            r.over_represented.is_empty(),
            "got {:?}",
            r.over_represented
        );
    }

    #[test]
    fn rows_are_ordered_by_count() {
        let b = Baseline::default();
        let shortlist: Vec<String> = (0..40)
            .map(|i| match i % 4 {
                0..=2 => "CSE".to_string(),
                _ => "ECE".to_string(),
            })
            .collect();
        let r = analyse(&shortlist, &b, 40, 100).unwrap();
        assert_eq!(r.rows.value[0].branch, "CSE");
        assert!(r.rows.value[0].count > r.rows.value[1].count);
    }

    #[test]
    fn lift_is_absent_without_a_branch_baseline() {
        let b = Baseline::from_values(&[8.0; 10]);
        let shortlist: Vec<String> = (0..40).map(|_| "CSE".to_string()).collect();
        let r = analyse(&shortlist, &b, 40, 100).unwrap();
        assert!(r.rows.value[0].lift.is_none());
        assert!(r.over_represented.is_empty());
    }

    #[test]
    fn report_carries_its_sample() {
        let b = Baseline::default();
        let shortlist: Vec<String> = (0..30).map(|_| "CSE".to_string()).collect();
        let r = analyse(&shortlist, &b, 30, 200).unwrap();
        assert_eq!(r.rows.matched, 30);
        assert_eq!(r.rows.total, 200);
        assert!(r.statement.contains("30 of 200"));
    }
}
