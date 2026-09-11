//! Name normalisation and comparison.
//!
//! Every function here is tuned for **precision over recall**. A refused match
//! costs a student one unknown name; a wrong match tells them a friend was
//! rejected when they were shortlisted, or attaches someone else's CGPA to them.
//! Those harms are not comparable, so when the evidence is thin we decline.

use once_cell::sync::Lazy;
use rphonetic::{DoubleMetaphone, Encoder};
use unicode_normalization::UnicodeNormalization;

/// Per-token Jaro-Winkler floor for a typo to count as the same token.
pub const TOKEN_SIMILARITY: f64 = 0.92;

static METAPHONE: Lazy<DoubleMetaphone> = Lazy::new(|| DoubleMetaphone::new(Some(6)));

/// Lowercases, strips accents, removes punctuation and digits, collapses runs of
/// whitespace.
///
/// Fixes the double-space in `Srihitha  Komatineni` and the trailing initial in
/// `Subash Athithya.m` in a single pass.
pub fn normalise(raw: &str) -> String {
    let decomposed: String = raw.nfkd().collect();
    let mut out = String::with_capacity(decomposed.len());
    let mut prev_space = true;
    for ch in decomposed.chars() {
        let c = if ch.is_alphabetic() {
            ch.to_ascii_lowercase()
        } else {
            ' '
        };
        if c == ' ' {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out.trim().to_string()
}

/// Tokens of two or more characters. Single characters are initials and are
/// handled separately — they never carry a match on their own.
pub fn tokens(raw: &str) -> Vec<String> {
    normalise(raw)
        .split(' ')
        .filter(|t| t.len() > 1)
        .map(|t| t.to_string())
        .collect()
}

/// Single-letter initials present in the name.
pub fn initials(raw: &str) -> Vec<char> {
    normalise(raw)
        .split(' ')
        .filter(|t| t.chars().count() == 1)
        .filter_map(|t| t.chars().next())
        .collect()
}

/// Order-independent signature: tokens sorted and rejoined.
///
/// This is the workhorse. It equates `SRIKANTH A` with `A Srikanth`, and
/// `sivaprathish sivamoorhty` with `Sivamoorhty Sivaprathish`.
pub fn name_key(raw: &str) -> String {
    let mut t = tokens(raw);
    t.sort();
    t.join(" ")
}

/// Double Metaphone signature for a whole name, order-independent.
///
/// Catches transliteration variance: `Sivamoorthy` / `Sivamoorhty`,
/// `Krishna` / `Krisna`.
pub fn phonetic_key(raw: &str) -> String {
    let mut codes: Vec<String> = tokens(raw)
        .iter()
        .map(|t| {
            let c = METAPHONE.encode(t);
            if c.is_empty() {
                t.clone()
            } else {
                c
            }
        })
        .collect();
    codes.sort();
    codes.join(" ")
}

/// How two names relate. Ordering is meaningful: a later variant is stronger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NameRelation {
    /// Nothing in common worth acting on.
    Unrelated,
    /// Same sound, different spelling, or a typo within tolerance.
    Fuzzy,
    /// One name's tokens are a strict subset of the other's — a missing middle
    /// name. Only ever `Probable`, because `Shresth Kumar Gupta` and
    /// `Niraj Kumar Gupta` also share two tokens.
    Subset,
    /// Identical token signature.
    Exact,
}

/// Compares two names without deciding anything. The caller applies the
/// ambiguity gate; this function only reports the relationship.
pub fn compare(a: &str, b: &str) -> NameRelation {
    let (ka, kb) = (name_key(a), name_key(b));
    if ka.is_empty() || kb.is_empty() {
        return NameRelation::Unrelated;
    }
    if ka == kb {
        return NameRelation::Exact;
    }

    let ta = tokens(a);
    let tb = tokens(b);

    // Subset: every token of the shorter name appears in the longer one, and at
    // least two full tokens agree. Requiring two blocks single-surname
    // collisions like `Soham Sinha` against `Akash Sinha`.
    let (short, long) = if ta.len() <= tb.len() {
        (&ta, &tb)
    } else {
        (&tb, &ta)
    };
    if short.len() >= 2 && short.iter().all(|t| long.contains(t)) {
        return NameRelation::Subset;
    }

    // Phonetic equality across the whole name.
    if phonetic_key(a) == phonetic_key(b) {
        return NameRelation::Fuzzy;
    }

    // Same token count, every token within typo distance.
    if ta.len() == tb.len() && ta.len() >= 2 {
        let mut used = vec![false; tb.len()];
        let mut all = true;
        for t in &ta {
            let found = tb
                .iter()
                .enumerate()
                .position(|(i, u)| !used[i] && strsim::jaro_winkler(t, u) >= TOKEN_SIMILARITY);
            match found {
                Some(i) => used[i] = true,
                None => {
                    all = false;
                    break;
                }
            }
        }
        if all {
            return NameRelation::Fuzzy;
        }
    }

    NameRelation::Unrelated
}

/// Whether an initial set is compatible with a token set.
///
/// `SRIKANTH A` is compatible with `Srikanth Arumugam` because `A` matches the
/// first letter of a token the other name has. Compatibility alone never makes a
/// match — it only avoids vetoing one.
pub fn initials_compatible(inits: &[char], other_tokens: &[String]) -> bool {
    inits.iter().all(|i| {
        other_tokens
            .iter()
            .any(|t| t.chars().next().map(|c| c == *i).unwrap_or(false))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalisation_collapses_real_world_mess() {
        assert_eq!(normalise("Srihitha  Komatineni"), "srihitha komatineni");
        assert_eq!(normalise("  SRIKANTH A "), "srikanth a");
        assert_eq!(normalise("Subash Athithya.m"), "subash athithya m");
        assert_eq!(normalise("Cse(ai&ml)"), "cse ai ml");
    }

    #[test]
    fn name_key_is_order_independent() {
        assert_eq!(name_key("Monish D"), name_key("D Monish"));
        assert_eq!(
            name_key("sivaprathish sivamoorhty"),
            name_key("Sivamoorhty  SIVAPRATHISH")
        );
    }

    #[test]
    fn single_letter_initials_are_excluded_from_the_key() {
        // `SRIKANTH A` keys on `srikanth` alone, so it can meet `A Srikanth`.
        assert_eq!(name_key("SRIKANTH A"), "srikanth");
        assert_eq!(initials("SRIKANTH A"), vec!['a']);
    }

    #[test]
    fn exact_relation_for_identical_signatures() {
        assert_eq!(compare("Monish D", "D  monish"), NameRelation::Exact);
    }

    #[test]
    fn subset_relation_for_missing_middle_name() {
        assert_eq!(
            compare("Sujal Chhajed", "Sujal Sanjay Chhajed"),
            NameRelation::Subset
        );
        assert_eq!(
            compare("Sumit Patnaik", "Sumit Kumar Patnaik"),
            NameRelation::Subset
        );
    }

    #[test]
    fn shared_surname_alone_is_unrelated() {
        // These are different people. The engine must not link them.
        assert_eq!(
            compare("Soham Sinha", "Akash Sinha"),
            NameRelation::Unrelated
        );
        assert_eq!(
            compare("Sushant Banerjee", "Ishita Banerjee"),
            NameRelation::Unrelated
        );
        assert_eq!(
            compare("Tanisha Gupta", "Atharv Gupta"),
            NameRelation::Unrelated
        );
    }

    #[test]
    fn the_dangerous_pair_is_only_ever_subset_never_exact() {
        // `Shresth Kumar Gupta` vs `Niraj Kumar Gupta` are different students
        // who share two tokens. Subset is the strongest this may return, and
        // Subset alone is never allowed to name a person.
        let r = compare("Shresth Kumar Gupta", "Niraj Kumar Gupta");
        assert_ne!(r, NameRelation::Exact);
        assert!(r < NameRelation::Exact);
    }

    #[test]
    fn phonetic_catches_transliteration_variants() {
        assert_eq!(
            compare("Sivaprathish Sivamoorthy", "Sivaprathish Sivamoorhty"),
            NameRelation::Fuzzy
        );
    }

    #[test]
    fn typos_within_tolerance_are_fuzzy() {
        assert_eq!(compare("Dhruv Sahni", "Dhruv Sahnii"), NameRelation::Fuzzy);
    }

    #[test]
    fn unrelated_names_stay_unrelated() {
        assert_eq!(
            compare("Monish D", "Yuvaraj Ulaganathan"),
            NameRelation::Unrelated
        );
        assert_eq!(compare("", "Monish D"), NameRelation::Unrelated);
    }

    #[test]
    fn initials_compatibility_is_permissive_but_not_a_match() {
        let toks = vec!["srikanth".to_string(), "arumugam".to_string()];
        assert!(initials_compatible(&['a'], &toks));
        assert!(!initials_compatible(&['z'], &toks));
    }

    #[test]
    fn comparison_is_symmetric() {
        let pairs = [
            ("Sujal Chhajed", "Sujal Sanjay Chhajed"),
            ("Soham Sinha", "Akash Sinha"),
            ("Monish D", "D Monish"),
        ];
        for (a, b) in pairs {
            assert_eq!(compare(a, b), compare(b, a), "asymmetric for {a} / {b}");
        }
    }
}
