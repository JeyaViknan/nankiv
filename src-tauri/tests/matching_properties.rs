//! Property tests for the matcher.
//!
//! One invariant carries the whole design: **no ambiguous input may ever
//! produce a confident match.** A refused match costs a student one unknown
//! name; a wrong match tells them a friend was rejected when they were
//! shortlisted. Those harms are not comparable.
//!
//! Example-based tests can only cover the collisions we thought of. These
//! generate them.

use nankiv_core::identity::matcher::{CorpusIndex, MatchOutcome};
use nankiv_core::identity::name;
use nankiv_core::model::Confidence;
use proptest::prelude::*;

/// Name-ish tokens: lowercase words of plausible length.
fn token() -> impl Strategy<Value = String> {
    "[a-z]{3,10}".prop_map(|s| s)
}

fn full_name() -> impl Strategy<Value = String> {
    (token(), token()).prop_map(|(a, b)| format!("{a} {b}"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(400))]

    /// Two different students sharing a name can never be resolved to one.
    #[test]
    fn duplicate_names_are_never_confidently_matched(nm in full_name()) {
        let mut idx = CorpusIndex::new();
        idx.insert(&nm, "23AAA0001");
        idx.insert(&nm, "23BBB0002");

        let out = idx.match_name(&nm);
        prop_assert!(
            matches!(out, MatchOutcome::Ambiguous { .. }),
            "identical names under different students must stay ambiguous, got {out:?}"
        );
        prop_assert!(!out.confidence().can_name_person());
    }

    /// A shared surname alone must never link two people.
    #[test]
    fn a_shared_surname_never_names_a_person(
        first_a in token(),
        first_b in token(),
        surname in token(),
    ) {
        prop_assume!(first_a != first_b);
        prop_assume!(first_a != surname && first_b != surname);

        let mut idx = CorpusIndex::new();
        idx.insert(&format!("{first_a} {surname}"), "23AAA0001");

        let out = idx.match_name(&format!("{first_b} {surname}"));
        prop_assert!(
            !out.confidence().can_name_person(),
            "{first_b} {surname} must not resolve to {first_a} {surname}, got {out:?}"
        );
    }

    /// The middle-name case: `A B C` vs `A C` is at most Probable, because
    /// `Shresth Kumar Gupta` and `Niraj Kumar Gupta` are indistinguishable from
    /// `Sujal Chhajed` and `Sujal Sanjay Chhajed` by score alone.
    #[test]
    fn a_missing_middle_name_never_exceeds_probable(
        first in token(),
        middle in token(),
        last in token(),
    ) {
        prop_assume!(first != middle && middle != last && first != last);

        let mut idx = CorpusIndex::new();
        idx.insert(&format!("{first} {middle} {last}"), "23AAA0001");

        let out = idx.match_name(&format!("{first} {last}"));
        prop_assert!(
            out.confidence() <= Confidence::Probable,
            "subset matches must cap at Probable, got {out:?}"
        );
    }

    /// A unique exact name always resolves — precision must not cost us the
    /// cases that are genuinely unambiguous.
    #[test]
    fn a_unique_exact_name_always_resolves(nm in full_name()) {
        let mut idx = CorpusIndex::new();
        idx.insert(&nm, "23AAA0001");
        idx.insert("zzzz yyyy", "23BBB0002");

        let out = idx.match_name(&nm);
        prop_assert!(
            matches!(out, MatchOutcome::Matched { .. }),
            "an unambiguous name should resolve, got {out:?}"
        );
        prop_assert_eq!(out.confidence(), Confidence::High);
    }

    /// Normalisation must not change the answer: spacing, case and punctuation
    /// are noise in this data, not signal.
    #[test]
    fn formatting_noise_does_not_change_the_result(a in token(), b in token()) {
        let mut idx = CorpusIndex::new();
        idx.insert(&format!("{a} {b}"), "23AAA0001");

        for variant in [
            format!("{a} {b}"),
            format!("{} {}", a.to_uppercase(), b.to_uppercase()),
            format!("  {a}   {b}  "),
            format!("{a}.{b}"),
            format!("{b} {a}"),
        ] {
            let out = idx.match_name(&variant);
            prop_assert!(
                matches!(out, MatchOutcome::Matched { .. }),
                "variant {variant:?} should still match"
            );
        }
    }

    /// Comparison is symmetric — a relationship cannot depend on argument order.
    #[test]
    fn name_comparison_is_symmetric(a in full_name(), b in full_name()) {
        prop_assert_eq!(name::compare(&a, &b), name::compare(&b, &a));
    }

    /// A name always matches itself.
    #[test]
    fn comparison_is_reflexive(a in full_name()) {
        prop_assert_eq!(name::compare(&a, &a), name::NameRelation::Exact);
    }

    /// The normalised key is stable under re-normalisation.
    #[test]
    fn the_name_key_is_idempotent(a in full_name()) {
        let once = name::name_key(&a);
        prop_assert_eq!(name::name_key(&once), once);
    }

    /// Three or more students under one name stay ambiguous — the `Naveen` case,
    /// which occurs three times in the real reference sheet.
    #[test]
    fn many_way_collisions_stay_ambiguous(nm in full_name(), n in 3usize..8) {
        let mut idx = CorpusIndex::new();
        for i in 0..n {
            idx.insert(&nm, &format!("23AAA{i:04}"));
        }
        let out = idx.match_name(&nm);
        match out {
            MatchOutcome::Ambiguous { ref candidates } => {
                prop_assert_eq!(candidates.len(), n);
            }
            other => prop_assert!(false, "expected ambiguity, got {other:?}"),
        }
    }

    /// The same student listed twice under different spellings is one student,
    /// not a collision — ambiguity is about distinct people.
    #[test]
    fn one_student_under_two_spellings_is_not_ambiguous(a in token(), b in token()) {
        let mut idx = CorpusIndex::new();
        idx.insert(&format!("{a} {b}"), "23AAA0001");
        idx.insert(&format!("{}  {}", a.to_uppercase(), b), "23AAA0001");

        let out = idx.match_name(&format!("{a} {b}"));
        prop_assert!(
            matches!(out, MatchOutcome::Matched { .. }),
            "same payload is not a collision, got {out:?}"
        );
    }

    /// An empty or whitespace-only query never matches anything.
    #[test]
    fn blank_queries_never_match(pad in "[ \t]{0,8}") {
        let mut idx = CorpusIndex::new();
        idx.insert("Someone Real", "23AAA0001");
        prop_assert_eq!(idx.match_name(&pad), MatchOutcome::NoMatch);
    }
}

/// Confidence ordering must stay consistent with the permissions it grants.
#[test]
fn confidence_permissions_follow_its_ordering() {
    let all = [
        Confidence::Unresolved,
        Confidence::Probable,
        Confidence::High,
        Confidence::Verified,
    ];
    for w in all.windows(2) {
        assert!(w[0] < w[1]);
        // Naming permission is monotonic: if the weaker may name, so may the
        // stronger.
        if w[0].can_name_person() {
            assert!(w[1].can_name_person());
        }
        if w[0].can_aggregate() {
            assert!(w[1].can_aggregate());
        }
    }
    // And nothing below High may ever name a person.
    assert!(!Confidence::Unresolved.can_name_person());
    assert!(!Confidence::Probable.can_name_person());
}
