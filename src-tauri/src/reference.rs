//! The bundled reference pack.
//!
//! nankiv ships knowing the cohort. Asking every student to go and find the
//! CGPA sheet before the app could tell them anything was friction they had no
//! way to anticipate and no reason to accept — the maintainer already holds the
//! sheet, so making two thousand people each re-supply it is a tax with no
//! payer.
//!
//! What ships is the *resolved* result, not the source sheets: registration
//! number, name, CGPA and branch, plus the Neo ID links that name matching
//! could establish safely. The source also carries personal email addresses,
//! phone numbers, gender, dates of birth and resume links; none of those are
//! read by the generator and none have a field here to land in.
//!
//! Regenerate with `cargo run --bin build_reference` after updating `Global/`.

use crate::model::{Confidence, Identifier, NeoId, RegNo};
use crate::store::{Store, StoreError};
use serde::Deserialize;

/// The pack, compiled into the binary so there is no file to lose.
const PACK: &str = include_str!("../reference/pack.json");

const SEEDED_KEY: &str = "reference_pack_version";

#[derive(Debug, Deserialize)]
struct Pack {
    version: u32,
    #[serde(default)]
    sources: Vec<String>,
    /// `[neo id, reg no, confidence]`
    #[serde(default)]
    edges: Vec<[String; 3]>,
    /// `[identifier, name key, confidence]`
    #[serde(default)]
    name_edges: Vec<[String; 3]>,
    /// name key -> the spelling to show
    #[serde(default)]
    names: std::collections::BTreeMap<String, String>,
    /// `[reg no, cgpa, branch]`
    #[serde(default)]
    academics: Vec<(String, Option<f64>, Option<String>)>,
    #[serde(default)]
    baseline_cgpa: Vec<f64>,
    #[serde(default)]
    baseline_branch: Vec<String>,
}

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct SeedReport {
    pub students: usize,
    pub academics: usize,
    pub already_seeded: bool,
}

fn parse_confidence(s: &str) -> Confidence {
    match s {
        "verified" => Confidence::Verified,
        "high" => Confidence::High,
        "probable" => Confidence::Probable,
        _ => Confidence::Unresolved,
    }
}

/// Loads the bundled pack into an empty or out-of-date database.
///
/// Idempotent: every write is an upsert or ignore, and the pack version is
/// recorded so a later launch does no work. A newer pack in a newer build seeds
/// over the top without disturbing anything the student imported themselves —
/// their own files produce `Verified` links, which outrank anything here.
pub fn seed(store: &Store) -> Result<SeedReport, StoreError> {
    let pack: Pack = match serde_json::from_str(PACK) {
        Ok(p) => p,
        Err(e) => {
            // A malformed pack must not stop the application from starting; the
            // student can still import their own data.
            eprintln!("bundled reference pack could not be read: {e}");
            return Ok(SeedReport::default());
        }
    };

    if let Some(v) = store.meta(SEEDED_KEY)? {
        if v.parse::<u32>().unwrap_or(0) >= pack.version {
            return Ok(SeedReport {
                already_seeded: true,
                ..Default::default()
            });
        }
    }

    for (key, display) in &pack.names {
        store.remember_spelling(key, display)?;
    }

    for [ident, key, conf] in &pack.name_edges {
        let id = NeoId::parse(ident)
            .map(Identifier::NeoId)
            .or_else(|| RegNo::parse(ident).map(Identifier::RegNo));
        if let Some(id) = id {
            store.add_edge(
                &id,
                &Identifier::NameKey(key.clone()),
                parse_confidence(conf),
                "bundled",
            )?;
        }
    }

    let mut students = 0;
    for [neo, reg, conf] in &pack.edges {
        if let (Some(n), Some(r)) = (NeoId::parse(neo), RegNo::parse(reg)) {
            store.add_edge(
                &Identifier::NeoId(n),
                &Identifier::RegNo(r),
                parse_confidence(conf),
                "bundled",
            )?;
            students += 1;
        }
    }

    let mut academics = 0;
    for (reg, cgpa, branch) in &pack.academics {
        if let Some(r) = RegNo::parse(reg) {
            store.upsert_academic(&r, *cgpa, branch.as_deref(), "bundled")?;
            academics += 1;
        }
    }

    // The baseline is anonymous by construction: values with no identifier
    // beside them, which is what makes a cutoff legible without exposing anyone.
    let cohort = "23";
    for (i, cgpa) in pack.baseline_cgpa.iter().enumerate() {
        store.add_baseline(
            cohort,
            Some(*cgpa),
            pack.baseline_branch.get(i).map(|s| s.as_str()),
        )?;
    }

    store.set_meta(SEEDED_KEY, &pack.version.to_string())?;
    if !pack.sources.is_empty() {
        store.set_meta("reference_pack_sources", &pack.sources.join(", "))?;
    }

    Ok(SeedReport {
        students,
        academics,
        already_seeded: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_pack_is_valid_json_with_content() {
        let pack: Pack = serde_json::from_str(PACK).expect("pack parses");
        assert_eq!(pack.version, 1);
        assert!(!pack.academics.is_empty(), "pack should carry academics");
        assert!(!pack.edges.is_empty(), "pack should resolve some students");
    }

    #[test]
    fn seeding_populates_an_empty_database() {
        let s = Store::open_in_memory().unwrap();
        let r = seed(&s).unwrap();
        assert!(!r.already_seeded);
        assert!(r.students > 500, "got {}", r.students);
        assert!(s.academic_count().unwrap() > 2000);
    }

    #[test]
    fn seeding_twice_does_no_further_work() {
        let s = Store::open_in_memory().unwrap();
        seed(&s).unwrap();
        let before = s.edge_count().unwrap();
        let again = seed(&s).unwrap();
        assert!(again.already_seeded);
        assert_eq!(s.edge_count().unwrap(), before);
    }

    #[test]
    fn a_seeded_database_can_answer_a_shortlist() {
        use crate::analytics::Baseline;
        let s = Store::open_in_memory().unwrap();
        seed(&s).unwrap();
        let identity = crate::engine::build_graph(&s).unwrap();

        // Every resolvable student should reach academic data.
        let resolvable = identity
            .neo_to_reg
            .keys()
            .filter(|n| {
                identity
                    .reg_of_neo(n)
                    .and_then(|r| s.academic(r).ok().flatten())
                    .is_some()
            })
            .count();
        assert!(resolvable > 500, "only {resolvable} reach academics");

        let (cgpa, _) = s.baseline(Some("23")).unwrap();
        let base = Baseline::from_values(&cgpa);
        assert!(!base.is_empty(), "baseline should be seeded");
        // The measured cohort shape: roughly a quarter below 8.5.
        assert!((0.15..0.35).contains(&base.share_below(8.5)));
    }

    #[test]
    fn the_pack_carries_no_contact_details() {
        // Structural check on what actually ships. The generator never reads
        // these fields, and this fails loudly if that ever changes.
        for needle in ["@gmail", "@vitstudent", "drive.google", "docs.google"] {
            assert!(
                !PACK.contains(needle),
                "bundled pack must not contain {needle}"
            );
        }
        // Ten-digit phone numbers.
        let phones = regex::Regex::new(r"\b[6-9]\d{9}\b").unwrap();
        assert!(
            !phones.is_match(PACK),
            "bundled pack must not contain phone numbers"
        );
    }
}
