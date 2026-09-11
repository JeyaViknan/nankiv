//! Identity resolution: normalisation, matching, and the identifier graph.

pub mod graph;
pub mod matcher;
pub mod name;

pub use graph::{Component, Edge, IdentityGraph};
pub use matcher::{Candidate, CorpusIndex, MatchOutcome};
