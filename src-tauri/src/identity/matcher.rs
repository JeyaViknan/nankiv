//! The matcher, and the ambiguity gate.
//!
//! The gate is the most important rule in the application. If the best candidate
//! does not beat the runner-up by a clear margin, **the match is refused**. No
//! tie-breaking, no "most likely".
//!
//! This is not conservatism for its own sake. In the sample corpus
//! `Shresth Kumar Gupta` and `Niraj Kumar Gupta` score identically to
//! `Sujal Chhajed` and `Sujal Sanjay Chhajed` — one pair is two different
//! people, the other is one person, and no score can separate them. There were
//! 95 such collisions.

use super::name::{self, NameRelation};
use crate::model::Confidence;

/// A candidate name from the reference corpus.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    /// Normalised name key.
    pub key: String,
    /// Original spelling, for display.
    pub display: String,
    /// Opaque payload — usually a registration number.
    pub payload: String,
}

/// The outcome of matching one query name against the corpus.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchOutcome {
    /// Exactly one candidate, strong enough to act on.
    Matched {
        candidate: Candidate,
        confidence: Confidence,
    },
    /// Two or more candidates the evidence cannot separate. Deliberately a
    /// first-class outcome rather than an error: the interface shows these so
    /// the user can choose, and the engine never chooses for them.
    Ambiguous { candidates: Vec<Candidate> },
    /// Nothing close enough.
    NoMatch,
}

impl MatchOutcome {
    pub fn confidence(&self) -> Confidence {
        match self {
            MatchOutcome::Matched { confidence, .. } => *confidence,
            _ => Confidence::Unresolved,
        }
    }
}

/// Maps a name relation onto the confidence it may justify.
///
/// `Subset` caps at `Probable` and can therefore never name a person — this is
/// what stops `Shresth Kumar Gupta` becoming `Niraj Kumar Gupta` on screen.
fn relation_confidence(r: NameRelation) -> Confidence {
    match r {
        NameRelation::Exact => Confidence::High,
        NameRelation::Fuzzy => Confidence::Probable,
        NameRelation::Subset => Confidence::Probable,
        NameRelation::Unrelated => Confidence::Unresolved,
    }
}

/// Matches `query` against `corpus`, refusing anything ambiguous.
///
/// The corpus is expected to be pre-filtered to plausible candidates (see
/// [`CorpusIndex`]); this function applies the decision rules.
pub fn match_name(query: &str, corpus: &[Candidate]) -> MatchOutcome {
    let qk = name::name_key(query);
    if qk.is_empty() {
        return MatchOutcome::NoMatch;
    }

    let mut scored: Vec<(NameRelation, &Candidate)> = corpus
        .iter()
        .map(|c| (name::compare(query, &c.display), c))
        .filter(|(r, _)| *r != NameRelation::Unrelated)
        .collect();

    if scored.is_empty() {
        return MatchOutcome::NoMatch;
    }

    // Strongest relation first.
    scored.sort_by_key(|(relation, _)| std::cmp::Reverse(*relation));
    let best_relation = scored[0].0;

    // Every candidate tied at the best relation.
    let tied: Vec<&Candidate> = scored
        .iter()
        .filter(|(r, _)| *r == best_relation)
        .map(|(_, c)| *c)
        .collect();

    // Distinct payloads matter, not distinct spellings: the same student listed
    // twice under slightly different spellings is not an ambiguity.
    let mut payloads: Vec<&str> = tied.iter().map(|c| c.payload.as_str()).collect();
    payloads.sort_unstable();
    payloads.dedup();

    if payloads.len() > 1 {
        return MatchOutcome::Ambiguous {
            candidates: tied.into_iter().cloned().collect(),
        };
    }

    let winner = tied[0].clone();
    let confidence = relation_confidence(best_relation);
    if confidence == Confidence::Unresolved {
        return MatchOutcome::NoMatch;
    }

    MatchOutcome::Matched {
        candidate: winner,
        confidence,
    }
}

/// An index over the reference corpus that narrows candidates cheaply before the
/// expensive comparisons run.
#[derive(Debug, Default)]
pub struct CorpusIndex {
    by_exact: std::collections::HashMap<String, Vec<Candidate>>,
    by_phonetic: std::collections::HashMap<String, Vec<Candidate>>,
    by_token: std::collections::HashMap<String, Vec<usize>>,
    all: Vec<Candidate>,
}

impl CorpusIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, display: &str, payload: &str) {
        let key = name::name_key(display);
        if key.is_empty() {
            return;
        }
        let c = Candidate {
            key: key.clone(),
            display: display.trim().to_string(),
            payload: payload.to_string(),
        };
        let idx = self.all.len();
        self.all.push(c.clone());
        self.by_exact.entry(key).or_default().push(c.clone());
        self.by_phonetic
            .entry(name::phonetic_key(display))
            .or_default()
            .push(c);
        for t in name::tokens(display) {
            self.by_token.entry(t).or_default().push(idx);
        }
    }

    pub fn len(&self) -> usize {
        self.all.len()
    }

    pub fn is_empty(&self) -> bool {
        self.all.is_empty()
    }

    /// Plausible candidates for a query: exact key, phonetic key, or any shared
    /// token. Cheap filter; the decision rules run afterwards.
    pub fn candidates_for(&self, query: &str) -> Vec<Candidate> {
        let mut out: Vec<Candidate> = Vec::new();
        let mut seen: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();

        let mut push = |c: &Candidate, out: &mut Vec<Candidate>| {
            if seen.insert((c.key.clone(), c.payload.clone())) {
                out.push(c.clone());
            }
        };

        if let Some(v) = self.by_exact.get(&name::name_key(query)) {
            for c in v {
                push(c, &mut out);
            }
        }
        if let Some(v) = self.by_phonetic.get(&name::phonetic_key(query)) {
            for c in v {
                push(c, &mut out);
            }
        }
        for t in name::tokens(query) {
            if let Some(ids) = self.by_token.get(&t) {
                for &i in ids {
                    let c = self.all[i].clone();
                    push(&c, &mut out);
                }
            }
        }
        out
    }

    /// Convenience: narrow then decide.
    pub fn match_name(&self, query: &str) -> MatchOutcome {
        let cands = self.candidates_for(query);
        match_name(query, &cands)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx(pairs: &[(&str, &str)]) -> CorpusIndex {
        let mut i = CorpusIndex::new();
        for (n, p) in pairs {
            i.insert(n, p);
        }
        i
    }

    #[test]
    fn exact_unique_name_matches_at_high_confidence() {
        let i = idx(&[("Monish D", "23BCE1473"), ("Dhruv Sahni", "23BCE1633")]);
        match i.match_name("monish  d") {
            MatchOutcome::Matched {
                candidate,
                confidence,
            } => {
                assert_eq!(candidate.payload, "23BCE1473");
                assert_eq!(confidence, Confidence::High);
            }
            other => panic!("expected a match, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_names_are_refused_not_guessed() {
        // Three real students share the key `naveen` in the reference sheet.
        let i = idx(&[
            ("Naveen", "23BAI1069"),
            ("Naveen", "23BCE1063"),
            ("Naveen", "23BPS1117"),
        ]);
        match i.match_name("Naveen") {
            MatchOutcome::Ambiguous { candidates } => assert_eq!(candidates.len(), 3),
            other => panic!("must refuse, got {other:?}"),
        }
    }

    #[test]
    fn the_gupta_collision_never_produces_a_named_match() {
        // Two different students. Whatever the engine decides, it must not be
        // allowed to name either of them.
        let i = idx(&[
            ("Niraj Kumar Gupta", "23AAA0001"),
            ("Ankit Kumar Gupta", "23AAA0002"),
        ]);
        let out = i.match_name("Shresth Kumar Gupta");
        assert!(
            !out.confidence().can_name_person(),
            "must not name a person from a shared-surname collision, got {out:?}"
        );
    }

    #[test]
    fn missing_middle_name_matches_only_as_probable() {
        let i = idx(&[("Sujal Sanjay Chhajed", "23XYZ0001")]);
        let out = i.match_name("Sujal Chhajed");
        assert_eq!(out.confidence(), Confidence::Probable);
        assert!(
            !out.confidence().can_name_person(),
            "probable may aggregate but never name"
        );
    }

    #[test]
    fn shared_surname_alone_is_no_match() {
        let i = idx(&[("Akash Sinha", "23AAA0001")]);
        assert_eq!(i.match_name("Soham Sinha"), MatchOutcome::NoMatch);
    }

    #[test]
    fn same_student_listed_twice_is_not_ambiguous() {
        // Same payload under two spellings — one student, not a collision.
        let i = idx(&[("Monish D", "23BCE1473"), ("Monish  D ", "23BCE1473")]);
        match i.match_name("Monish D") {
            MatchOutcome::Matched { candidate, .. } => {
                assert_eq!(candidate.payload, "23BCE1473")
            }
            other => panic!("expected a match, got {other:?}"),
        }
    }

    #[test]
    fn exact_beats_fuzzy_rather_than_tying_with_it() {
        let i = idx(&[
            ("Dhruv Sahni", "23BCE1633"),
            ("Dhruv Sahnii", "23BCE9999"), // typo-adjacent different student
        ]);
        match i.match_name("Dhruv Sahni") {
            MatchOutcome::Matched {
                candidate,
                confidence,
            } => {
                assert_eq!(candidate.payload, "23BCE1633");
                assert_eq!(confidence, Confidence::High);
            }
            other => panic!("exact should win outright, got {other:?}"),
        }
    }

    #[test]
    fn empty_and_unknown_queries_return_no_match() {
        let i = idx(&[("Monish D", "23BCE1473")]);
        assert_eq!(i.match_name(""), MatchOutcome::NoMatch);
        assert_eq!(i.match_name("   "), MatchOutcome::NoMatch);
        assert_eq!(i.match_name("Completely Different"), MatchOutcome::NoMatch);
    }

    #[test]
    fn empty_corpus_matches_nothing() {
        let i = CorpusIndex::new();
        assert!(i.is_empty());
        assert_eq!(i.match_name("Anyone"), MatchOutcome::NoMatch);
    }

    #[test]
    fn index_narrows_but_does_not_lose_the_right_candidate() {
        let mut i = CorpusIndex::new();
        for n in 0..500 {
            i.insert(&format!("Filler Person{n}"), &format!("23FIL{n:04}"));
        }
        i.insert("Yuvaraj U", "23BCE1481");
        match i.match_name("Yuvaraj U") {
            MatchOutcome::Matched { candidate, .. } => {
                assert_eq!(candidate.payload, "23BCE1481")
            }
            other => panic!("expected a match, got {other:?}"),
        }
    }
}
