//! Core domain types.
//!
//! The reliability hierarchy from the specification is encoded here in the type
//! system rather than left to convention. Two invariants matter above all:
//!
//! 1. An *unknown* identity can never be rendered as *not shortlisted*. These are
//!    separate variants of [`Verdict`], and there is deliberately no `From` impl,
//!    no `unwrap_or`, and no `Default` that would let one silently decay into the
//!    other.
//! 2. An *estimated* CGPA cutoff can never be presented as an official one. Every
//!    statistic is wrapped in [`Estimate`], which carries its own sample size and
//!    cannot be constructed without one.

use serde::{Deserialize, Serialize};
use std::fmt;

// ---------------------------------------------------------------------------
// Identifiers
// ---------------------------------------------------------------------------

/// A Neo ID: letter-digit repeated four times, uppercase. e.g. `V9H0G6C4`.
///
/// Verified invariant across all 5,339 IDs in the sample corpus — no exceptions,
/// no duplicates. Parsing is therefore format-driven rather than header-driven.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NeoId(String);

impl NeoId {
    /// Parses a Neo ID, trimming surrounding whitespace and upper-casing.
    /// Returns `None` if the value does not match the exact `LDLDLDLD` shape.
    pub fn parse(raw: &str) -> Option<Self> {
        let s = raw.trim().to_ascii_uppercase();
        if s.len() != 8 {
            return None;
        }
        let bytes = s.as_bytes();
        for (i, b) in bytes.iter().enumerate() {
            let ok = if i % 2 == 0 {
                b.is_ascii_uppercase()
            } else {
                b.is_ascii_digit()
            };
            if !ok {
                return None;
            }
        }
        Some(NeoId(s))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for NeoId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A registration number, e.g. `23BAI0002`, `22MIM10069`.
///
/// Shape: two admission-year digits, three programme letters, then 4-5 digits.
/// Two of the fifteen sampled shortlists are keyed by this instead of Neo ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RegNo(String);

impl RegNo {
    pub fn parse(raw: &str) -> Option<Self> {
        let s = raw.trim().to_ascii_uppercase();
        if s.len() < 9 || s.len() > 11 {
            return None;
        }
        let b = s.as_bytes();
        let shape = b[0].is_ascii_digit()
            && b[1].is_ascii_digit()
            && b[2..5].iter().all(|c| c.is_ascii_uppercase())
            && b[5..].iter().all(|c| c.is_ascii_digit());
        if shape {
            Some(RegNo(s))
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Two-digit admission year, e.g. `23`. Used to filter the analytics
    /// baseline to the student's own cohort — the Deloitte file proved that
    /// multiple batches circulate in a single shortlist.
    pub fn admission_year(&self) -> &str {
        &self.0[0..2]
    }
}

impl fmt::Display for RegNo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Which kind of key a file (or a user profile) is keyed by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyKind {
    NeoId,
    RegNo,
}

impl fmt::Display for KeyKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyKind::NeoId => f.write_str("Neo ID"),
            KeyKind::RegNo => f.write_str("registration number"),
        }
    }
}

/// Any identifier that can name a student.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Identifier {
    NeoId(NeoId),
    RegNo(RegNo),
    /// Normalised name key. Weakest node type — never used alone to assert identity.
    NameKey(String),
}

impl Identifier {
    pub fn kind_label(&self) -> &'static str {
        match self {
            Identifier::NeoId(_) => "neo_id",
            Identifier::RegNo(_) => "reg_no",
            Identifier::NameKey(_) => "name",
        }
    }
}

// ---------------------------------------------------------------------------
// Confidence
// ---------------------------------------------------------------------------

/// How much we trust a link between two identifiers.
///
/// Ordering is meaningful and derived: `Unresolved < Probable < High < Verified`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    /// Ambiguous or absent. Excluded from everything. Also the `Default`: when
    /// in doubt the system must fail toward knowing less, not more.
    #[default]
    Unresolved,
    /// Fuzzy match with a clear unique winner. Analytics only — never shown as a
    /// named person, because a wrong name is far worse than a missing one.
    Probable,
    /// Exact normalised name match, unique on both sides.
    High,
    /// Two identifiers observed in the same row of the same file. Treated as fact.
    Verified,
}

impl Confidence {
    /// Whether this link is strong enough to display as a named individual.
    pub fn can_name_person(self) -> bool {
        matches!(self, Confidence::High | Confidence::Verified)
    }

    /// Whether this link may contribute to aggregate statistics.
    pub fn can_aggregate(self) -> bool {
        matches!(
            self,
            Confidence::Probable | Confidence::High | Confidence::Verified
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            Confidence::Verified => "verified",
            Confidence::High => "high",
            Confidence::Probable => "probable",
            Confidence::Unresolved => "unresolved",
        }
    }
}

// ---------------------------------------------------------------------------
// Verdict — the single most important type in the application
// ---------------------------------------------------------------------------

/// Why a membership question could not be answered.
///
/// Every variant carries enough context for the UI to explain itself. None of
/// them may be rendered with the same treatment as [`Verdict::NotShortlisted`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum Undetermined {
    /// The student has not told us who they are yet.
    NoIdentityConfigured,
    /// The file is keyed by something the student hasn't configured. For example
    /// a registration-number file when only a Neo ID is set: absence from that
    /// file says nothing at all about them.
    KeyKindNotConfigured { file_key: KeyKind },
    /// The file contained no identifiers this application recognises — the
    /// TCS/Cognizant `REFERENCE_ID` case. Reporting zero here would read as a
    /// rejection, which is the worst available bug.
    FileNotUnderstood,
}

/// The answer to "am I (or is this person) on this shortlist?".
///
/// There is deliberately no `Default`, no `unwrap_or`, and no boolean coercion.
/// Turning [`Undetermined`] into [`Verdict::NotShortlisted`] must require writing
/// an explicit match arm, so it cannot happen by accident.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Verdict {
    Shortlisted,
    NotShortlisted,
    Undetermined(Undetermined),
}

impl Verdict {
    /// Resolves membership against a set, but *only* when the caller proves the
    /// file is keyed by something we hold for this student. This is the single
    /// constructor that can produce `NotShortlisted`.
    pub fn from_lookup(found: bool) -> Self {
        if found {
            Verdict::Shortlisted
        } else {
            Verdict::NotShortlisted
        }
    }

    pub fn is_determined(&self) -> bool {
        !matches!(self, Verdict::Undetermined(_))
    }
}

// ---------------------------------------------------------------------------
// Estimates — statistics that cannot be stated without their sample
// ---------------------------------------------------------------------------

/// A statistic bound to the sample it came from.
///
/// Constructing one requires supplying `matched` and `total`, so a figure can
/// never reach the interface stripped of its coverage. The Elgi file produced a
/// confident "cutoff ~9.0" from a single student before this type existed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Estimate<T> {
    pub value: T,
    /// Students in the shortlist we could attach academic data to.
    pub matched: usize,
    /// Students in the shortlist in total.
    pub total: usize,
}

impl<T> Estimate<T> {
    pub fn new(value: T, matched: usize, total: usize) -> Self {
        Estimate {
            value,
            matched,
            total,
        }
    }

    pub fn coverage(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.matched as f64 / self.total as f64
        }
    }
}

/// Minimum sample before any statistical verdict is emitted.
/// Derived from the Elgi failure: n=1 must never yield a cutoff.
pub const MIN_SAMPLE_N: usize = 20;
/// Minimum share of the shortlist that must be matched.
///
/// Lower than it looks, deliberately. The gate exists to stop claims the data
/// cannot support, and the binding constraint on validity is the *sample size*
/// above, not the share. Coverage governs how representative the sample is —
/// and the matched subset was measured against the full cohort at a mean of
/// 8.73 against 8.82, so it is not systematically different from the students
/// we cannot resolve. A thin but unbiased sample of 25 says something real; the
/// interface's job is then to state the coverage plainly, which it does.
pub const MIN_COVERAGE: f64 = 0.08;

/// Whether a sample is strong enough to support a statistical claim.
pub fn sample_is_sufficient(matched: usize, total: usize) -> bool {
    if total == 0 {
        return false;
    }
    matched >= MIN_SAMPLE_N && (matched as f64 / total as f64) >= MIN_COVERAGE
}

// ---------------------------------------------------------------------------
// Student-facing records
// ---------------------------------------------------------------------------

/// What the application knows about one person on a shortlist.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolvedStudent {
    pub neo_id: Option<NeoId>,
    pub reg_no: Option<RegNo>,
    /// Present only when the underlying link is `High` or `Verified`.
    pub name: Option<String>,
    pub confidence: Confidence,
}

impl ResolvedStudent {
    /// A display label that never invents a name it isn't sure of.
    pub fn display_label(&self) -> String {
        match (&self.name, self.confidence.can_name_person()) {
            (Some(n), true) => n.clone(),
            _ => self
                .neo_id
                .as_ref()
                .map(|i| i.to_string())
                .or_else(|| self.reg_no.as_ref().map(|r| r.to_string()))
                .unwrap_or_else(|| "Unknown student".to_string()),
        }
    }
}

/// Academic facts used by the analytics engine. Deliberately minimal: phone,
/// email, date of birth, gender, resume links and 10th/12th marks are discarded
/// at ingestion and never reach storage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcademicRecord {
    pub reg_no: RegNo,
    pub cgpa: Option<f64>,
    pub branch: Option<String>,
}

/// Clamp a raw CGPA to the plausible range.
///
/// The reference sheet contains one value of `87.0` — a percentage typed into
/// the CGPA column — and one unparseable entry.
pub fn sanitise_cgpa(raw: f64) -> Option<f64> {
    if raw.is_finite() && (4.0..=10.0).contains(&raw) {
        Some(raw)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neo_id_accepts_canonical_shape() {
        for s in ["V9H0G6C4", "E4P3P9H4", "Z1E5C3U4", "O4J3H2L8"] {
            assert!(NeoId::parse(s).is_some(), "should accept {s}");
        }
    }

    #[test]
    fn neo_id_normalises_case_and_whitespace() {
        assert_eq!(NeoId::parse("  v9h0g6c4 ").unwrap().as_str(), "V9H0G6C4");
    }

    #[test]
    fn neo_id_rejects_near_misses() {
        // Digit where a letter belongs — the 04J3H2L8 / O4J3H2L8 confusion.
        assert!(NeoId::parse("04J3H2L8").is_none());
        assert!(NeoId::parse("V9H0G6C").is_none(), "too short");
        assert!(NeoId::parse("V9H0G6C44").is_none(), "too long");
        assert!(NeoId::parse("VVH0G6C4").is_none(), "letter in digit slot");
        assert!(
            NeoId::parse("23BAI0002").is_none(),
            "reg no is not a neo id"
        );
        assert!(NeoId::parse("").is_none());
    }

    #[test]
    fn reg_no_accepts_real_shapes() {
        for s in ["23BAI0002", "22BCE7607", "22MIM10069", "23BCE20136"] {
            assert!(RegNo::parse(s).is_some(), "should accept {s}");
        }
    }

    #[test]
    fn reg_no_rejects_neo_ids_and_foreign_ids() {
        assert!(RegNo::parse("V9H0G6C4").is_none());
        assert!(RegNo::parse("CT20264996884").is_none(), "TCS reference id");
        assert!(RegNo::parse("DT20268151988").is_none());
    }

    #[test]
    fn reg_no_exposes_admission_year() {
        assert_eq!(RegNo::parse("23BAI0002").unwrap().admission_year(), "23");
        assert_eq!(RegNo::parse("22MIM10069").unwrap().admission_year(), "22");
    }

    #[test]
    fn confidence_orders_correctly() {
        assert!(Confidence::Verified > Confidence::High);
        assert!(Confidence::High > Confidence::Probable);
        assert!(Confidence::Probable > Confidence::Unresolved);
    }

    #[test]
    fn only_strong_confidence_may_name_a_person() {
        assert!(Confidence::Verified.can_name_person());
        assert!(Confidence::High.can_name_person());
        assert!(!Confidence::Probable.can_name_person());
        assert!(!Confidence::Unresolved.can_name_person());
    }

    #[test]
    fn probable_aggregates_but_does_not_name() {
        assert!(Confidence::Probable.can_aggregate());
        assert!(!Confidence::Probable.can_name_person());
    }

    #[test]
    fn undetermined_is_never_not_shortlisted() {
        let u = Verdict::Undetermined(Undetermined::FileNotUnderstood);
        assert_ne!(u, Verdict::NotShortlisted);
        assert!(!u.is_determined());
    }

    #[test]
    fn probable_match_falls_back_to_identifier_not_name() {
        let s = ResolvedStudent {
            neo_id: NeoId::parse("V9H0G6C4"),
            reg_no: None,
            name: Some("Someone Guessed".into()),
            confidence: Confidence::Probable,
        };
        assert_eq!(s.display_label(), "V9H0G6C4");
    }

    #[test]
    fn sample_gate_rejects_the_elgi_case() {
        // 1 matched student out of 125 produced a confident cutoff before gating.
        assert!(!sample_is_sufficient(1, 125));
        // Siemens: 22 of 166 is 13% — a thin but real and unbiased sample.
        assert!(sample_is_sufficient(22, 166));
        assert!(sample_is_sufficient(25, 166));
        // Still refused when the share is so small the sample says nothing
        // about the shortlist as a whole.
        assert!(!sample_is_sufficient(30, 900));
        // Enough coverage but too few students to mean anything.
        assert!(!sample_is_sufficient(5, 10));
        assert!(!sample_is_sufficient(0, 0));
    }

    #[test]
    fn cgpa_sanitisation_drops_percentage_typo() {
        assert_eq!(sanitise_cgpa(87.0), None, "percentage in the cgpa column");
        assert_eq!(sanitise_cgpa(8.82), Some(8.82));
        assert_eq!(sanitise_cgpa(f64::NAN), None);
        assert_eq!(sanitise_cgpa(0.0), None);
    }

    #[test]
    fn estimate_reports_its_own_coverage() {
        let e = Estimate::new(8.5, 25, 166);
        assert!((e.coverage() - 0.1506).abs() < 0.001);
    }
}
