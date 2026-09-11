//! The identity graph.
//!
//! Three identifier types exist — Neo ID, registration number, name — and
//! different files link different pairs. Modelling this as a table would force a
//! primary key that no file consistently provides. Modelling it as a graph does
//! not: each identifier is a node, each co-occurrence is an edge, and a
//! connected component is a student.
//!
//! **Names are not nodes.** This is the central design decision here. A name is
//! an *attribute* of a student, not an identifier for one. Making a name a merge
//! point means the three real students named `Naveen` collapse into a single
//! corrupted component the moment any file mentions them. So the union-find
//! operates only over Neo IDs and registration numbers, and names hang off
//! components as labels. Resolving a name back to a person is then a separate
//! question, answered by the matcher, which can refuse when it is ambiguous.

use crate::model::{Confidence, Identifier, NeoId, RegNo};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// A single claim that two identifiers belong to the same student.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub a: Identifier,
    pub b: Identifier,
    pub confidence: Confidence,
    /// Where this claim came from, for auditing and for undoing a bad import.
    pub source: String,
}

/// A resolved student: everything the graph believes about one person.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Component {
    pub neo_ids: BTreeSet<NeoId>,
    pub reg_nos: BTreeSet<RegNo>,
    /// Name keys observed for this student, with the confidence each was seen at.
    pub names: BTreeMap<String, Confidence>,
    /// The weakest link holding this component together. A component is only as
    /// trustworthy as its flimsiest edge.
    pub confidence: Confidence,
    /// Set when the component acquired two different registration numbers, which
    /// means a merge was wrong.
    pub conflicted: bool,
}

impl Component {
    /// The best display name, or `None` if nothing is strong enough to show.
    pub fn best_name(&self) -> Option<&str> {
        if self.conflicted {
            return None;
        }
        self.names
            .iter()
            .filter(|(_, c)| c.can_name_person())
            .max_by_key(|(_, c)| **c)
            .map(|(n, _)| n.as_str())
    }
}

/// Union-find over identifiers, with names attached as attributes.
#[derive(Debug, Default)]
pub struct IdentityGraph {
    index: HashMap<Identifier, usize>,
    nodes: Vec<Identifier>,
    parent: Vec<usize>,
    rank: Vec<usize>,
    /// Weakest edge confidence within each component root.
    conf: Vec<Confidence>,
    conflicted: Vec<bool>,
    /// Name attributes: node index -> (name key -> confidence).
    node_names: HashMap<usize, BTreeMap<String, Confidence>>,
    /// Original display spelling for a name key.
    display_names: HashMap<String, String>,
    edges: Vec<Edge>,
}

impl IdentityGraph {
    pub fn new() -> Self {
        Self::default()
    }

    fn intern(&mut self, id: &Identifier) -> usize {
        if let Some(&i) = self.index.get(id) {
            return i;
        }
        let i = self.nodes.len();
        self.nodes.push(id.clone());
        self.parent.push(i);
        self.rank.push(0);
        self.conf.push(Confidence::Verified);
        self.conflicted.push(false);
        self.index.insert(id.clone(), i);
        i
    }

    fn find(&mut self, mut x: usize) -> usize {
        while self.parent[x] != x {
            self.parent[x] = self.parent[self.parent[x]];
            x = self.parent[x];
        }
        x
    }

    /// Records the human-readable spelling behind a normalised name key.
    pub fn remember_spelling(&mut self, key: &str, display: &str) {
        self.display_names
            .entry(key.to_string())
            .or_insert_with(|| display.trim().to_string());
    }

    pub fn spelling_of(&self, key: &str) -> Option<&str> {
        self.display_names.get(key).map(|s| s.as_str())
    }

    /// Attaches a name to an identifier without merging anything.
    ///
    /// Two students sharing a name stay two students; the name simply labels
    /// both, and the matcher decides later whether it can pick between them.
    pub fn attach_name(&mut self, id: &Identifier, name_key: &str, confidence: Confidence) {
        if confidence == Confidence::Unresolved || name_key.is_empty() {
            return;
        }
        if matches!(id, Identifier::NameKey(_)) {
            return; // A name cannot label a name.
        }
        let i = self.intern(id);
        let entry = self.node_names.entry(i).or_default();
        let slot = entry.entry(name_key.to_string()).or_insert(confidence);
        if confidence > *slot {
            *slot = confidence;
        }
    }

    /// Adds a claim that two identifiers are the same student.
    ///
    /// Name identifiers are rejected: use [`attach_name`](Self::attach_name).
    /// Merging two components takes the *weaker* of their confidences.
    pub fn link(&mut self, a: &Identifier, b: &Identifier, confidence: Confidence, source: &str) {
        if confidence == Confidence::Unresolved {
            return; // An unresolved claim is not a claim.
        }
        // Names never merge. Route them to the attribute store instead, so a
        // shared name cannot glue two students together.
        match (a, b) {
            (Identifier::NameKey(k), other) | (other, Identifier::NameKey(k)) => {
                if !matches!(other, Identifier::NameKey(_)) {
                    self.attach_name(other, k, confidence);
                }
                return;
            }
            _ => {}
        }

        let (ia, ib) = (self.intern(a), self.intern(b));
        self.edges.push(Edge {
            a: a.clone(),
            b: b.clone(),
            confidence,
            source: source.to_string(),
        });

        let (ra, rb) = (self.find(ia), self.find(ib));
        if ra == rb {
            self.conf[ra] = self.conf[ra].min(confidence);
            return;
        }

        let merged_conf = self.conf[ra].min(self.conf[rb]).min(confidence);
        let merged_conflict = self.conflicted[ra] || self.conflicted[rb];

        let root = if self.rank[ra] < self.rank[rb] {
            self.parent[ra] = rb;
            rb
        } else if self.rank[ra] > self.rank[rb] {
            self.parent[rb] = ra;
            ra
        } else {
            self.parent[rb] = ra;
            self.rank[ra] += 1;
            ra
        };
        self.conf[root] = merged_conf;
        self.conflicted[root] = merged_conflict;
    }

    /// Materialises every component.
    ///
    /// A component holding two different registration numbers is flagged as
    /// conflicted and demoted to `Unresolved` — the graph must be able to say
    /// "I was wrong" rather than silently carrying a corrupted student.
    pub fn components(&mut self) -> Vec<Component> {
        let mut grouped: BTreeMap<usize, Component> = BTreeMap::new();
        let n = self.nodes.len();

        for i in 0..n {
            let root = self.find(i);
            let node = self.nodes[i].clone();
            let names = self.node_names.get(&i).cloned().unwrap_or_default();
            let entry = grouped.entry(root).or_default();
            match node {
                Identifier::NeoId(x) => {
                    entry.neo_ids.insert(x);
                }
                Identifier::RegNo(x) => {
                    entry.reg_nos.insert(x);
                }
                Identifier::NameKey(_) => continue,
            }
            for (k, c) in names {
                let slot = entry.names.entry(k).or_insert(c);
                if c > *slot {
                    *slot = c;
                }
            }
        }

        for (root, comp) in grouped.iter_mut() {
            comp.confidence = self.conf[*root];
            if comp.reg_nos.len() > 1 {
                comp.conflicted = true;
                comp.confidence = Confidence::Unresolved;
            } else {
                comp.conflicted = self.conflicted[*root];
            }
            let c = comp.confidence;
            for v in comp.names.values_mut() {
                *v = (*v).min(c);
            }
        }

        grouped.into_values().collect()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Edges grouped by source file, for the "what did this import teach us"
    /// message and for undoing an import.
    pub fn edges_by_source(&self) -> BTreeMap<&str, usize> {
        let mut m = BTreeMap::new();
        for e in &self.edges {
            *m.entry(e.source.as_str()).or_insert(0) += 1;
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn neo(s: &str) -> Identifier {
        Identifier::NeoId(NeoId::parse(s).expect("valid neo id"))
    }
    fn reg(s: &str) -> Identifier {
        Identifier::RegNo(RegNo::parse(s).expect("valid reg no"))
    }
    fn key(s: &str) -> String {
        crate::identity::name::name_key(s)
    }

    #[test]
    fn a_verified_row_creates_one_student() {
        let mut g = IdentityGraph::new();
        g.link(
            &neo("E2S8L9L8"),
            &reg("23BCE1473"),
            Confidence::Verified,
            "tredence",
        );
        g.attach_name(&neo("E2S8L9L8"), &key("Monish D"), Confidence::High);

        let comps = g.components();
        assert_eq!(comps.len(), 1);
        let c = &comps[0];
        assert_eq!(c.neo_ids.len(), 1);
        assert_eq!(c.reg_nos.len(), 1);
        assert_eq!(c.confidence, Confidence::Verified);
        assert!(!c.conflicted);
        assert_eq!(c.best_name(), Some(key("Monish D").as_str()));
    }

    #[test]
    fn two_students_sharing_a_name_stay_separate() {
        // The bug this design exists to prevent. Three real students share the
        // key `naveen`; a name must never merge them.
        let mut g = IdentityGraph::new();
        g.link(
            &neo("V9H0G6C4"),
            &reg("23AAA0001"),
            Confidence::Verified,
            "f",
        );
        g.link(
            &neo("C5U6K1E7"),
            &reg("23BBB0002"),
            Confidence::Verified,
            "f",
        );
        g.attach_name(&neo("V9H0G6C4"), &key("Naveen"), Confidence::High);
        g.attach_name(&neo("C5U6K1E7"), &key("Naveen"), Confidence::High);

        let comps = g.components();
        assert_eq!(comps.len(), 2, "a shared name must not merge students");
        assert!(comps.iter().all(|c| !c.conflicted));
        assert!(comps.iter().all(|c| c.best_name().is_some()));
    }

    #[test]
    fn linking_through_a_name_identifier_does_not_merge() {
        // Even when a caller passes a NameKey to `link`, it is routed to the
        // attribute store rather than becoming a merge point.
        let mut g = IdentityGraph::new();
        let nm = Identifier::NameKey(key("Naveen"));
        g.link(&neo("V9H0G6C4"), &nm, Confidence::High, "a");
        g.link(&neo("C5U6K1E7"), &nm, Confidence::High, "b");
        assert_eq!(g.components().len(), 2);
    }

    #[test]
    fn transitive_identifier_links_join_across_files() {
        let mut g = IdentityGraph::new();
        g.link(
            &neo("V9H0G6C4"),
            &reg("23BAI0001"),
            Confidence::High,
            "accenture",
        );
        g.link(&reg("23BAI0001"), &neo("V9H0G6C4"), Confidence::High, "hpe");
        let comps = g.components();
        assert_eq!(comps.len(), 1);
        assert_eq!(comps[0].neo_ids.len(), 1);
        assert_eq!(comps[0].reg_nos.len(), 1);
    }

    #[test]
    fn a_component_is_only_as_strong_as_its_weakest_edge() {
        let mut g = IdentityGraph::new();
        g.link(
            &neo("V9H0G6C4"),
            &reg("23BAI0001"),
            Confidence::Verified,
            "a",
        );
        g.link(
            &reg("23BAI0001"),
            &neo("C5U6K1E7"),
            Confidence::Probable,
            "b",
        );
        let comps = g.components();
        assert_eq!(comps[0].confidence, Confidence::Probable);
    }

    #[test]
    fn two_reg_numbers_in_one_component_is_flagged_and_demoted() {
        let mut g = IdentityGraph::new();
        g.link(
            &neo("V9H0G6C4"),
            &reg("23AAA0001"),
            Confidence::Probable,
            "guess",
        );
        g.link(
            &neo("V9H0G6C4"),
            &reg("23BBB0002"),
            Confidence::Probable,
            "guess",
        );
        g.attach_name(&neo("V9H0G6C4"), &key("Kumar Gupta"), Confidence::High);

        let comps = g.components();
        assert_eq!(comps.len(), 1);
        assert!(comps[0].conflicted, "must notice the contradiction");
        assert_eq!(comps[0].confidence, Confidence::Unresolved);
        assert!(
            comps[0].best_name().is_none(),
            "a conflicted student must never be named"
        );
    }

    #[test]
    fn unresolved_links_are_ignored_entirely() {
        let mut g = IdentityGraph::new();
        g.link(
            &neo("V9H0G6C4"),
            &reg("23BAI0001"),
            Confidence::Unresolved,
            "x",
        );
        assert_eq!(g.edge_count(), 0);
        assert_eq!(g.components().len(), 0);
    }

    #[test]
    fn probable_names_do_not_surface() {
        let mut g = IdentityGraph::new();
        g.link(
            &neo("V9H0G6C4"),
            &reg("23BAI0001"),
            Confidence::Verified,
            "f",
        );
        g.attach_name(&neo("V9H0G6C4"), &key("Maybe Person"), Confidence::Probable);
        assert!(g.components()[0].best_name().is_none());
    }

    #[test]
    fn a_stronger_name_claim_upgrades_a_weaker_one() {
        let mut g = IdentityGraph::new();
        g.link(
            &neo("V9H0G6C4"),
            &reg("23BAI0001"),
            Confidence::Verified,
            "f",
        );
        g.attach_name(&neo("V9H0G6C4"), &key("Real Person"), Confidence::Probable);
        g.attach_name(&neo("V9H0G6C4"), &key("Real Person"), Confidence::High);
        assert!(g.components()[0].best_name().is_some());
    }

    #[test]
    fn separate_students_stay_separate() {
        let mut g = IdentityGraph::new();
        g.link(
            &neo("V9H0G6C4"),
            &reg("23BAI0001"),
            Confidence::Verified,
            "f",
        );
        g.link(
            &neo("C5U6K1E7"),
            &reg("23BAI0002"),
            Confidence::Verified,
            "f",
        );
        assert_eq!(g.components().len(), 2);
    }

    #[test]
    fn repeated_identical_links_are_idempotent_for_components() {
        let mut g = IdentityGraph::new();
        for _ in 0..5 {
            g.link(
                &neo("V9H0G6C4"),
                &reg("23BAI0001"),
                Confidence::Verified,
                "f",
            );
        }
        assert_eq!(g.components().len(), 1);
    }

    #[test]
    fn sources_are_tracked() {
        let mut g = IdentityGraph::new();
        g.link(
            &neo("V9H0G6C4"),
            &reg("23BAI0001"),
            Confidence::Verified,
            "tredence",
        );
        g.link(
            &neo("C5U6K1E7"),
            &reg("23BAI0002"),
            Confidence::Verified,
            "tredence",
        );
        g.link(
            &neo("T2D4R9N9"),
            &reg("23BAI0003"),
            Confidence::High,
            "accenture",
        );
        let by = g.edges_by_source();
        assert_eq!(by.get("tredence"), Some(&2));
        assert_eq!(by.get("accenture"), Some(&1));
    }
}
