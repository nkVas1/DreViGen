//! Deterministic synthetic genealogies for benchmarks, property tests and demo data.
//!
//! DreViGen makes claims that only hold if they are measured: 50 000 people at 60 fps, a CRDT
//! merge inside a time and memory budget, record linkage that finds planted duplicates. None of
//! those can be measured against a random graph, because a random graph is not a genealogy. This
//! crate produces data with the shape of a real family tree — generational layers, pedigree
//! collapse, spouses who married in from nowhere, remarriage, and the brutal infant mortality
//! that fills a 19th-century parish register.
//!
//! Everything is seeded and platform-independent, so a benchmark run is comparable with one from
//! a year ago.
//!
//! ```
//! use drevigen_testkit::{SyntheticTree, TreeSpec};
//!
//! let tree = SyntheticTree::generate(TreeSpec::sized_for(5_000, 42));
//! let stats = tree.stats();
//!
//! assert!(stats.people >= 5_000);
//! assert!(stats.endogamous_unions > 0); // cousins married; the graph is a DAG, not a tree
//! ```
//!
//! The names are Russian on purpose. The name problems this project must solve — patronymics
//! derived irregularly from the father's given name, surnames that inflect for sex — are
//! invisible in data that reads `Person 00412`.

pub mod generate;
pub mod names;
pub mod rng;
pub mod sample;

pub use generate::{Family, FamilyId, Person, PersonId, SyntheticTree, TreeSpec, TreeStats};
pub use names::Sex;
pub use rng::Rng;
pub use sample::Subgraph;
