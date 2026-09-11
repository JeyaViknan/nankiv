//! Cutoff estimation.
//!
//! Two failures found while validating the specification against real files
//! shaped this module, and both are pinned by regression tests below:
//!
//! - **Siemens** (n=25, floor 8.94): a median of 9.28 with a floor at 8.94 is
//!   plainly a 9.0 bar, but a single student at 8.94 pushed a strict rule down
//!   to "8.5". Fixed by scaling the tolerance with sample size and by reporting
//!   the *observed floor* rather than only a snapped threshold.
//! - **Elgi** (n=1): produced a confident "cutoff ~9.0" from one data point.
//!   Fixed by the sufficiency gate in [`crate::model::sample_is_sufficient`],
//!   applied before this module is ever reached.

use super::{percentile, Baseline, Distribution};
use crate::model::Estimate;
use serde::{Deserialize, Serialize};

/// Thresholds a placement cell would plausibly set.
const LADDER: [f64; 6] = [9.5, 9.0, 8.5, 8.0, 7.5, 7.0];

/// The batch share below a threshold must be at least this for a floor there to
/// be meaningful. Prevents claiming a "cutoff at 7.0" when almost nobody in the
/// batch is below 7.0 anyway.
const MIN_BATCH_BELOW: f64 = 0.10;

/// How much higher the shortlist's floor must sit than the batch's before we
/// call it a preference rather than noise.
const SOFT_SHIFT: f64 = 0.15;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CutoffVerdict {
    /// A clean floor. `threshold` is the ladder value; `observed_floor` is the
    /// lowest CGPA actually seen, which is the honest number.
    HardCutoff { threshold: f64, observed_floor: f64 },
    /// Clearly higher than the batch, but without a clean floor.
    SoftPreference { observed_floor: f64 },
    /// Indistinguishable from the cohort — selection happened on something else.
    NoCgpaFilter,
}

/// A cutoff finding, inseparable from the sample that produced it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CutoffReport {
    pub verdict: Estimate<CutoffVerdict>,
    /// Per-threshold comparison, so the interface can show the working rather
    /// than asking the student to trust a verdict.
    pub comparison: Vec<ThresholdRow>,
    /// Plain-language statement. Always hedged — this is an estimate derived
    /// from a partial sample and must never read as an official cutoff.
    pub statement: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThresholdRow {
    pub threshold: f64,
    pub share_below_shortlist: f64,
    pub share_below_batch: f64,
    pub is_signal: bool,
}

/// Tolerance for students sitting below a candidate threshold.
///
/// At small n a single outlier must not veto an otherwise obvious floor; at
/// large n the rule tightens. One student in 25 is 4%, which a flat 2% rule
/// would have rejected — that was the Siemens bug.
fn tolerance(n: usize) -> f64 {
    if n == 0 {
        return 0.0;
    }
    (1.5 / n as f64).clamp(0.01, 0.06)
}

/// Estimates a cutoff. Callers must have already applied the sufficiency gate.
pub fn estimate(
    cgpas: &[f64],
    dist: &Distribution,
    baseline: &Baseline,
    matched: usize,
    total: usize,
) -> CutoffReport {
    let mut sorted: Vec<f64> = cgpas.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    let tol = tolerance(n);

    // Two different floors, for two different jobs. `observed_floor` is the
    // literal lowest CGPA we saw — the honest number to show a student. `p5` is
    // the robust one used for the soft-preference comparison, because with
    // partial coverage a single outlier should not characterise the shortlist.
    let observed_floor = sorted[0];
    let robust_floor = percentile(&sorted, 0.05);

    let mut comparison = Vec::new();
    let mut best: Option<f64> = None;

    for &t in LADDER.iter() {
        let below_list = sorted.iter().filter(|&&x| x < t - 1e-9).count() as f64 / n as f64;
        let below_batch = baseline.share_below(t);
        let is_signal = below_list <= tol && below_batch >= MIN_BATCH_BELOW;
        comparison.push(ThresholdRow {
            threshold: t,
            share_below_shortlist: below_list,
            share_below_batch: below_batch,
            is_signal,
        });
        // Ladder runs high to low, so the first signal is the highest threshold.
        if is_signal && best.is_none() {
            best = Some(t);
        }
    }

    let verdict = match best {
        Some(t) => CutoffVerdict::HardCutoff {
            threshold: t,
            observed_floor,
        },
        None => {
            let batch_floor = baseline
                .median()
                .map(|_| baseline_floor(baseline))
                .unwrap_or(0.0);
            if !baseline.is_empty() && robust_floor - batch_floor >= SOFT_SHIFT {
                CutoffVerdict::SoftPreference { observed_floor }
            } else {
                CutoffVerdict::NoCgpaFilter
            }
        }
    };

    let statement = phrase(&verdict, dist, matched, total);

    CutoffReport {
        verdict: Estimate::new(verdict, matched, total),
        comparison,
        statement,
    }
}

/// The batch's own 5th percentile, for the soft-preference comparison.
fn baseline_floor(baseline: &Baseline) -> f64 {
    // Reconstruct via the share_below curve rather than exposing the raw values.
    let mut lo = 4.0;
    let mut hi = 10.0;
    for _ in 0..40 {
        let mid = (lo + hi) / 2.0;
        if baseline.share_below(mid) < 0.05 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

/// Wording. Every phrase is explicitly an estimate from a partial sample.
fn phrase(v: &CutoffVerdict, dist: &Distribution, matched: usize, total: usize) -> String {
    let basis = format!("based on {matched} of {total} shortlisted students we could match");
    match v {
        CutoffVerdict::HardCutoff {
            threshold,
            observed_floor,
        } => format!(
            "Looks like a CGPA cutoff around {threshold:.1} — the lowest we found was {observed_floor:.2} ({basis}). This is an estimate, not an official cutoff."
        ),
        CutoffVerdict::SoftPreference { observed_floor } => format!(
            "No clean cutoff, but this shortlist skews high — median {:.2}, lowest found {observed_floor:.2} ({basis}). Likely a preference rather than a hard bar.",
            dist.median
        ),
        CutoffVerdict::NoCgpaFilter => format!(
            "No CGPA filter detected — this shortlist looks like the batch as a whole ({basis}). Selection probably happened on something else."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytics::baseline::test_support::batch_like;

    fn base() -> Baseline {
        Baseline::from_values(&batch_like())
    }

    fn report(v: &[f64]) -> CutoffReport {
        let d = Distribution::compute(v).unwrap();
        estimate(v, &d, &base(), v.len(), v.len() * 4)
    }

    #[test]
    fn siemens_shape_is_read_as_a_nine_bar_not_an_eight_five_bar() {
        // The real failure: median 9.28, floor 8.94, n=25. One student below 9.0
        // must not drag the answer down to 8.5.
        let mut v = vec![8.94];
        for i in 0..24 {
            v.push(9.0 + (i as f64 % 8.0) * 0.08);
        }
        let r = report(&v);
        match r.verdict.value {
            CutoffVerdict::HardCutoff { threshold, .. } => {
                assert_eq!(threshold, 9.0, "expected a 9.0 bar, got {threshold}");
            }
            other => panic!("expected a hard cutoff, got {other:?}"),
        }
    }

    #[test]
    fn the_observed_floor_is_reported_not_just_the_snapped_threshold() {
        let mut v = vec![8.94];
        for i in 0..24 {
            v.push(9.0 + (i as f64 % 8.0) * 0.08);
        }
        let r = report(&v);
        if let CutoffVerdict::HardCutoff { observed_floor, .. } = r.verdict.value {
            assert!(
                observed_floor < 9.0,
                "the honest floor is the one we saw, got {observed_floor}"
            );
        } else {
            panic!("expected a hard cutoff");
        }
        assert!(r.statement.contains("estimate"));
    }

    #[test]
    fn a_clean_nine_pointer_list_reads_as_a_nine_cutoff() {
        let v: Vec<f64> = (0..60).map(|i| 9.0 + (i % 20) as f64 * 0.04).collect();
        match report(&v).verdict.value {
            CutoffVerdict::HardCutoff { threshold, .. } => assert_eq!(threshold, 9.0),
            other => panic!("expected a hard cutoff, got {other:?}"),
        }
    }

    #[test]
    fn a_batch_shaped_list_reports_no_filter() {
        // Drawing from the batch itself must not manufacture a cutoff.
        let v = batch_like();
        let r = report(&v);
        assert_eq!(r.verdict.value, CutoffVerdict::NoCgpaFilter);
    }

    #[test]
    fn no_threshold_is_claimed_where_the_batch_is_already_above_it() {
        // 98% of the batch clears 8.0, so a shortlist with nobody below 8.0 is
        // not evidence of an 8.0 cutoff. This is the naive-rule trap.
        let v: Vec<f64> = (0..80).map(|i| 8.05 + (i % 40) as f64 * 0.02).collect();
        let r = report(&v);
        let row = r
            .comparison
            .iter()
            .find(|x| (x.threshold - 8.0).abs() < 1e-9)
            .unwrap();
        assert!(
            !row.is_signal,
            "8.0 must not be a signal: only {:.1}% of the batch is below it",
            row.share_below_batch * 100.0
        );
    }

    #[test]
    fn the_comparison_table_shows_its_working() {
        let v: Vec<f64> = (0..40).map(|i| 9.0 + (i % 10) as f64 * 0.05).collect();
        let r = report(&v);
        assert_eq!(r.comparison.len(), LADDER.len());
        for row in &r.comparison {
            assert!((0.0..=1.0).contains(&row.share_below_shortlist));
            assert!((0.0..=1.0).contains(&row.share_below_batch));
        }
    }

    #[test]
    fn tolerance_relaxes_for_small_samples_and_tightens_for_large() {
        assert!(tolerance(25) > tolerance(500));
        assert!(tolerance(25) >= 0.04, "one in 25 must be forgivable");
        assert!(tolerance(1000) <= 0.02);
        assert_eq!(tolerance(0), 0.0);
    }

    #[test]
    fn statements_never_claim_to_be_official() {
        let cases = vec![
            (0..60)
                .map(|i| 9.0 + (i % 20) as f64 * 0.04)
                .collect::<Vec<f64>>(),
            batch_like(),
        ];
        for v in cases {
            let s = report(&v).statement.to_lowercase();
            // It may *disclaim* officialness, but never assert it.
            assert!(
                !s.contains("the official") && !s.contains("is official"),
                "statement must not imply officialness: {s}"
            );
            assert!(
                s.contains("estimate") || s.contains("looks like") || s.contains("detected"),
                "statement must be hedged: {s}"
            );
        }
    }

    #[test]
    fn every_verdict_carries_its_sample() {
        let v: Vec<f64> = (0..30).map(|_| 9.2).collect();
        let d = Distribution::compute(&v).unwrap();
        let r = estimate(&v, &d, &base(), 30, 200);
        assert_eq!(r.verdict.matched, 30);
        assert_eq!(r.verdict.total, 200);
        assert!((r.verdict.coverage() - 0.15).abs() < 1e-9);
    }
}
