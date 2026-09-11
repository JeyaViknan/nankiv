//! The cohort baseline.
//!
//! A cutoff is a floor, and a floor is only visible against the batch it was cut
//! from. The baseline is the anonymous shape of the cohort: CGPA values and
//! branch labels with every identifier stripped.
//!
//! This is the one thing the application may ship pre-computed, because it
//! carries no names, no identifiers, and no ordering that could be re-linked to
//! a person.

use serde::{Deserialize, Serialize};

/// The anonymous distribution of a cohort.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Baseline {
    /// Sorted CGPA values. No identifiers, no order carrying meaning.
    sorted_cgpa: Vec<f64>,
    /// Canonical branch label counts.
    branch_counts: std::collections::BTreeMap<String, usize>,
    /// Admission year this baseline describes, e.g. `23`.
    pub cohort: Option<String>,
}

impl Baseline {
    pub fn from_values(values: &[f64]) -> Baseline {
        let mut v: Vec<f64> = values.to_vec();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Baseline {
            sorted_cgpa: v,
            branch_counts: Default::default(),
            cohort: None,
        }
    }

    pub fn with_branches(mut self, branches: &[String]) -> Self {
        for b in branches {
            *self.branch_counts.entry(b.clone()).or_insert(0) += 1;
        }
        self
    }

    pub fn is_empty(&self) -> bool {
        self.sorted_cgpa.is_empty()
    }

    pub fn n(&self) -> usize {
        self.sorted_cgpa.len()
    }

    /// Share of the cohort strictly below `threshold`.
    ///
    /// This is the number that makes cutoff detection work: 75.5% of the sample
    /// batch clears 8.5, so "most of the shortlist is above 8.5" means nothing
    /// on its own.
    pub fn share_below(&self, threshold: f64) -> f64 {
        if self.sorted_cgpa.is_empty() {
            return 0.0;
        }
        let count = self
            .sorted_cgpa
            .partition_point(|&x| x < threshold - f64::EPSILON);
        count as f64 / self.sorted_cgpa.len() as f64
    }

    /// Share of the cohort in a branch.
    pub fn branch_share(&self, branch: &str) -> f64 {
        let total: usize = self.branch_counts.values().sum();
        if total == 0 {
            return 0.0;
        }
        self.branch_counts.get(branch).copied().unwrap_or(0) as f64 / total as f64
    }

    pub fn has_branches(&self) -> bool {
        !self.branch_counts.is_empty()
    }

    pub fn median(&self) -> Option<f64> {
        if self.sorted_cgpa.is_empty() {
            None
        } else {
            Some(super::percentile(&self.sorted_cgpa, 0.5))
        }
    }
}

/// Shared fixtures. Compiled into tests and into the baseline-building script
/// so both exercise the same shape.
#[doc(hidden)]
pub mod test_support {
    /// A synthetic cohort whose shape mirrors the measured batch:
    /// mean ≈ 8.82, ~37% at or above 9.0, ~75% at or above 8.5, ~98% above 8.0.
    pub fn batch_like() -> Vec<f64> {
        let mut v = Vec::with_capacity(2000);
        // 2% below 8.0
        for i in 0..40 {
            v.push(6.6 + (i as f64 % 14.0) * 0.1);
        }
        // ~23% between 8.0 and 8.5
        for i in 0..460 {
            v.push(8.0 + (i as f64 % 50.0) * 0.01);
        }
        // ~38% between 8.5 and 9.0
        for i in 0..760 {
            v.push(8.5 + (i as f64 % 50.0) * 0.01);
        }
        // ~31% between 9.0 and 9.5
        for i in 0..620 {
            v.push(9.0 + (i as f64 % 50.0) * 0.01);
        }
        // ~6% at or above 9.5
        for i in 0..120 {
            v.push(9.5 + (i as f64 % 45.0) * 0.01);
        }
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_baseline_is_inert() {
        let b = Baseline::default();
        assert!(b.is_empty());
        assert_eq!(b.share_below(9.0), 0.0);
        assert_eq!(b.median(), None);
    }

    #[test]
    fn share_below_reproduces_the_measured_batch_shape() {
        let b = Baseline::from_values(&test_support::batch_like());
        // The three numbers that make naive threshold logic fail.
        let below_9 = b.share_below(9.0);
        let below_85 = b.share_below(8.5);
        let below_8 = b.share_below(8.0);
        assert!(
            (0.55..0.70).contains(&below_9),
            "expected ~63% below 9.0, got {below_9}"
        );
        assert!(
            (0.20..0.32).contains(&below_85),
            "expected ~25% below 8.5, got {below_85}"
        );
        assert!(below_8 < 0.05, "expected ~2% below 8.0, got {below_8}");
    }

    #[test]
    fn share_below_is_monotonic() {
        let b = Baseline::from_values(&test_support::batch_like());
        let mut prev = 0.0;
        for t in [7.0, 7.5, 8.0, 8.5, 9.0, 9.5, 10.0] {
            let s = b.share_below(t);
            assert!(s >= prev, "share_below must not decrease at {t}");
            prev = s;
        }
    }

    #[test]
    fn branch_share_is_proportional() {
        let b = Baseline::from_values(&[8.0, 9.0]).with_branches(&[
            "CSE".into(),
            "CSE".into(),
            "CSE".into(),
            "ECE".into(),
        ]);
        assert!((b.branch_share("CSE") - 0.75).abs() < 1e-9);
        assert!((b.branch_share("ECE") - 0.25).abs() < 1e-9);
        assert_eq!(b.branch_share("MECH"), 0.0);
        assert!(b.has_branches());
    }
}
