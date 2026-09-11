//! The analytics engine.
//!
//! The hard part is not computing statistics. It is refusing to compute them
//! when the sample cannot support it.
//!
//! A naive rule — "if most shortlisted students are above 8.5, the cutoff is
//! 8.5" — fires almost every time, because 75.5% of the batch already clears 8.5
//! and 98% clears 8.0. The right question is not *how many are above X* but
//! **has the bottom of the distribution been cut off relative to the batch**.
//! A cutoff is a floor, and a floor is only visible against a baseline.

pub mod baseline;
pub mod branch;
pub mod cutoff;

pub use baseline::Baseline;
pub use branch::{BranchLift, BranchReport};
pub use cutoff::{CutoffReport, CutoffVerdict};

use crate::model::{sample_is_sufficient, Estimate};
use serde::{Deserialize, Serialize};

/// Summary statistics for a set of CGPA values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Distribution {
    pub n: usize,
    pub min: f64,
    pub max: f64,
    pub median: f64,
    pub mean: f64,
    pub std_dev: f64,
    /// 5th percentile — the practical floor, less brittle than the raw minimum.
    pub p5: f64,
    pub p25: f64,
    pub p75: f64,
    /// Counts in 0.25-wide buckets, for the histogram.
    pub buckets: Vec<Bucket>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bucket {
    pub lower: f64,
    pub upper: f64,
    pub count: usize,
}

impl Distribution {
    /// Computes summary statistics. Returns `None` for an empty sample rather
    /// than inventing zeroes.
    pub fn compute(values: &[f64]) -> Option<Distribution> {
        if values.is_empty() {
            return None;
        }
        let mut v: Vec<f64> = values.to_vec();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = v.len();
        let mean = v.iter().sum::<f64>() / n as f64;
        let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;

        Some(Distribution {
            n,
            min: v[0],
            max: v[n - 1],
            median: percentile(&v, 0.50),
            mean,
            std_dev: var.sqrt(),
            p5: percentile(&v, 0.05),
            p25: percentile(&v, 0.25),
            p75: percentile(&v, 0.75),
            buckets: histogram(&v),
        })
    }
}

/// Nearest-rank percentile on an already-sorted slice.
pub fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    let idx = ((sorted.len() as f64) * q).floor() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

fn histogram(sorted: &[f64]) -> Vec<Bucket> {
    const STEP: f64 = 0.25;
    const LOW: f64 = 6.0;
    const HIGH: f64 = 10.0;
    let n = ((HIGH - LOW) / STEP) as usize;
    let mut buckets: Vec<Bucket> = (0..n)
        .map(|i| Bucket {
            lower: LOW + i as f64 * STEP,
            upper: LOW + (i + 1) as f64 * STEP,
            count: 0,
        })
        .collect();
    for &x in sorted {
        if x < LOW {
            buckets[0].count += 1;
            continue;
        }
        let i = (((x - LOW) / STEP).floor() as usize).min(n - 1);
        buckets[i].count += 1;
    }
    buckets
}

/// The complete analysis of one shortlist.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveAnalysis {
    /// Students on the shortlist in total.
    pub total_students: usize,
    /// Students we could attach academic data to.
    pub matched_students: usize,
    pub coverage: f64,
    /// True when the sample clears both the size and coverage bars.
    pub sufficient: bool,
    /// `None` when the sample is insufficient — the engine stays silent rather
    /// than reporting a shape it cannot support.
    pub cgpa: Option<Estimate<Distribution>>,
    pub cutoff: Option<CutoffReport>,
    pub branches: Option<BranchReport>,
    /// Where the student sits within this shortlist, when known and justified.
    pub your_percentile: Option<Estimate<f64>>,
}

impl DriveAnalysis {
    /// Builds the analysis, applying the sufficiency gate up front.
    pub fn build(
        cgpas: &[f64],
        branches: &[String],
        total_students: usize,
        baseline: &Baseline,
        your_cgpa: Option<f64>,
    ) -> DriveAnalysis {
        let matched = cgpas.len();
        let coverage = if total_students == 0 {
            0.0
        } else {
            matched as f64 / total_students as f64
        };
        let sufficient = sample_is_sufficient(matched, total_students);

        if !sufficient {
            return DriveAnalysis {
                total_students,
                matched_students: matched,
                coverage,
                sufficient: false,
                cgpa: None,
                cutoff: None,
                branches: None,
                your_percentile: None,
            };
        }

        let dist = Distribution::compute(cgpas);
        let cutoff = dist
            .as_ref()
            .map(|d| cutoff::estimate(cgpas, d, baseline, matched, total_students));
        let branch_report = branch::analyse(branches, baseline, matched, total_students);

        let your_percentile = your_cgpa.and_then(|mine| {
            let mut v = cgpas.to_vec();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            if v.is_empty() {
                return None;
            }
            let below = v.iter().filter(|&&x| x < mine).count();
            Some(Estimate::new(
                below as f64 / v.len() as f64,
                matched,
                total_students,
            ))
        });

        DriveAnalysis {
            total_students,
            matched_students: matched,
            coverage,
            sufficient: true,
            cgpa: dist.map(|d| Estimate::new(d, matched, total_students)),
            cutoff,
            branches: branch_report,
            your_percentile,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> Baseline {
        Baseline::from_values(&crate::analytics::baseline::test_support::batch_like())
    }

    #[test]
    fn empty_sample_has_no_distribution() {
        assert!(Distribution::compute(&[]).is_none());
    }

    #[test]
    fn distribution_matches_hand_computed_values() {
        let d = Distribution::compute(&[8.0, 9.0, 10.0]).unwrap();
        assert_eq!(d.n, 3);
        assert_eq!(d.min, 8.0);
        assert_eq!(d.max, 10.0);
        assert_eq!(d.median, 9.0);
        assert!((d.mean - 9.0).abs() < 1e-9);
    }

    #[test]
    fn histogram_totals_the_sample() {
        let v = vec![8.1, 8.4, 8.6, 9.0, 9.2, 9.9];
        let d = Distribution::compute(&v).unwrap();
        assert_eq!(d.buckets.iter().map(|b| b.count).sum::<usize>(), v.len());
    }

    #[test]
    fn histogram_clamps_values_below_range() {
        let d = Distribution::compute(&[4.5, 9.0]).unwrap();
        assert_eq!(d.buckets.iter().map(|b| b.count).sum::<usize>(), 2);
    }

    #[test]
    fn the_elgi_case_produces_no_analysis_at_all() {
        // One matched student out of 125. Before gating this produced a
        // confident "cutoff ~9.0" from a single data point.
        let a = DriveAnalysis::build(&[9.34], &["IT".into()], 125, &baseline(), None);
        assert!(!a.sufficient);
        assert!(a.cgpa.is_none(), "must not publish a distribution");
        assert!(a.cutoff.is_none(), "must not publish a cutoff");
        assert!(a.branches.is_none(), "must not publish branch lift");
        assert_eq!(a.matched_students, 1);
        assert_eq!(a.total_students, 125);
    }

    #[test]
    fn a_sufficient_sample_does_produce_an_analysis() {
        let cgpas: Vec<f64> = (0..40).map(|i| 8.5 + (i % 10) as f64 * 0.1).collect();
        let branches: Vec<String> = (0..40).map(|_| "CSE".to_string()).collect();
        let a = DriveAnalysis::build(&cgpas, &branches, 100, &baseline(), None);
        assert!(a.sufficient);
        assert!(a.cgpa.is_some());
        assert!(a.cutoff.is_some());
    }

    #[test]
    fn coverage_is_always_reported_even_when_gated() {
        let a = DriveAnalysis::build(&[9.0], &[], 200, &baseline(), None);
        assert!((a.coverage - 0.005).abs() < 1e-9);
    }

    #[test]
    fn percentile_places_the_student_correctly() {
        let cgpas: Vec<f64> = (0..40).map(|i| 8.0 + i as f64 * 0.05).collect();
        let branches: Vec<String> = (0..40).map(|_| "CSE".to_string()).collect();
        let a = DriveAnalysis::build(&cgpas, &branches, 100, &baseline(), Some(9.0));
        let p = a.your_percentile.expect("percentile");
        // 9.0 sits above 20 of the 40 values.
        assert!((p.value - 0.5).abs() < 0.05, "got {}", p.value);
    }

    #[test]
    fn percentile_carries_its_sample() {
        let cgpas: Vec<f64> = (0..40).map(|_| 9.0).collect();
        let branches: Vec<String> = (0..40).map(|_| "CSE".to_string()).collect();
        let a = DriveAnalysis::build(&cgpas, &branches, 100, &baseline(), Some(9.5));
        let p = a.your_percentile.unwrap();
        assert_eq!(p.matched, 40);
        assert_eq!(p.total, 100);
    }
}
