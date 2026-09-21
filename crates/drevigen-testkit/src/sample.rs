//! Extracting benchmark subgraphs of a fixed size.
//!
//! Layout algorithms are compared at many input sizes, which means drawing many subgraphs of
//! each size out of one large genealogy. *How* those subgraphs are cut decides whether the
//! comparison means anything.
//!
//! Racine (TU Wien, 2025, §6.1.2) makes the point sharply: the traversal must run over the
//! **undirected** graph. Walking the directed parent→child graph produces samples that are
//! artificially tree-like — fewer cycles, more single-parent nodes — and a benchmark built from
//! them flatters every algorithm under test, especially the ones that assume a tree. We adopt
//! the method, including its discipline that the *only* source of randomness is the starting
//! node; the traversal itself is deterministic.
//!
//! One deliberate deviation. Racine's model has no explicit family node, so adjacency there is
//! parent–child only. Ours does, and our renderer draws couples as a unit, so spouse links are
//! part of the adjacency here. A sample that cut couples in half would not resemble the input
//! our layout engine actually faces.

use std::collections::HashMap;

use crate::generate::{FamilyId, PersonId, SyntheticTree};
use crate::rng::Rng;

/// A connected subgraph of a [`SyntheticTree`], with dense indices for benchmarking.
#[derive(Debug, Clone)]
pub struct Subgraph {
    /// Selected people, in discovery order. Position in this vector is the dense index.
    pub people: Vec<PersonId>,
    /// Families with at least two selected members, so that each contributes a real edge.
    pub families: Vec<FamilyId>,
    /// Maps an original [`PersonId`] to its dense index within [`Subgraph::people`].
    pub index_of: HashMap<PersonId, u32>,
    /// The randomly chosen starting node.
    pub seed_person: PersonId,
}

impl Subgraph {
    /// Number of selected people.
    #[must_use]
    pub fn len(&self) -> usize {
        self.people.len()
    }

    /// Whether the subgraph is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.people.is_empty()
    }

    /// Whether a person is part of this subgraph.
    #[must_use]
    pub fn contains(&self, id: PersonId) -> bool {
        self.index_of.contains_key(&id)
    }
}

/// Extracts a connected subgraph of at most `size` people by breadth-first traversal of the
/// undirected graph, starting from a randomly chosen person.
///
/// Returns fewer than `size` people only when the connected component containing the start is
/// smaller than that. The same `seed` and `size` always produce the same subgraph.
///
/// # Examples
///
/// ```
/// use drevigen_testkit::{SyntheticTree, TreeSpec, sample};
///
/// let tree = SyntheticTree::generate(TreeSpec::sized_for(5_000, 1));
/// let a = sample::extract_subgraph(&tree, 120, 7);
/// let b = sample::extract_subgraph(&tree, 120, 7);
///
/// assert_eq!(a.len(), 120);
/// assert_eq!(a.people, b.people); // deterministic
/// ```
#[must_use]
pub fn extract_subgraph(tree: &SyntheticTree, size: usize, seed: u64) -> Subgraph {
    let mut rng = Rng::new(seed);

    if tree.people.is_empty() || size == 0 {
        return Subgraph {
            people: Vec::new(),
            families: Vec::new(),
            index_of: HashMap::new(),
            seed_person: PersonId(0),
        };
    }

    let start = PersonId(rng.below(tree.people.len() as u64) as u32);

    let mut visited = vec![false; tree.people.len()];
    let mut order: Vec<PersonId> = Vec::with_capacity(size.min(tree.people.len()));
    let mut queue: std::collections::VecDeque<PersonId> = std::collections::VecDeque::new();

    visited[start.0 as usize] = true;
    order.push(start);
    queue.push_back(start);

    let mut neighbours: Vec<PersonId> = Vec::new();
    while let Some(current) = queue.pop_front() {
        if order.len() >= size {
            break;
        }
        neighbours.clear();
        undirected_neighbours(tree, current, &mut neighbours);

        for &next in &neighbours {
            if visited[next.0 as usize] {
                continue;
            }
            visited[next.0 as usize] = true;
            order.push(next);
            queue.push_back(next);
            if order.len() >= size {
                break;
            }
        }
    }

    let index_of: HashMap<PersonId, u32> = order
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, u32::try_from(i).unwrap_or(u32::MAX)))
        .collect();

    // Induced families: keep one only when at least two of its members were selected, so every
    // retained family still carries a relationship rather than a dangling stub.
    let families = tree
        .families
        .iter()
        .filter(|family| {
            let present = family
                .husband
                .iter()
                .chain(family.wife.iter())
                .chain(family.children.iter())
                .filter(|id| index_of.contains_key(id))
                .count();
            present >= 2
        })
        .map(|family| family.id)
        .collect();

    Subgraph {
        people: order,
        families,
        index_of,
        seed_person: start,
    }
}

/// Collects the undirected neighbours of `person`: parents, children and spouses.
///
/// Written into a caller-supplied buffer because the traversal calls this once per dequeued
/// node and allocating a fresh vector each time dominates the runtime on large trees.
fn undirected_neighbours(tree: &SyntheticTree, person: PersonId, out: &mut Vec<PersonId>) {
    let p = tree.person(person);

    // Parents, through the family this person was born into.
    if let Some(fid) = p.child_of {
        let f = tree.family(fid);
        out.extend([f.husband, f.wife].into_iter().flatten());
    }

    // Spouses and children, through the families this person formed.
    for &fid in &p.spouse_in {
        let f = tree.family(fid);
        for partner in [f.husband, f.wife].into_iter().flatten() {
            if partner != person {
                out.push(partner);
            }
        }
        out.extend(f.children.iter().copied());
    }
}

/// Builds a benchmark corpus: `instances` subgraphs for every size in `sizes`.
///
/// Mirrors the design of Racine's evaluation, which used 200 sizes × 300 instances. Seeds are
/// derived from `base_seed`, the size and the instance number, so any single case in the corpus
/// can be reproduced in isolation.
#[must_use]
pub fn build_corpus(
    tree: &SyntheticTree,
    sizes: impl IntoIterator<Item = usize>,
    instances: usize,
    base_seed: u64,
) -> Vec<Subgraph> {
    let mut corpus = Vec::new();
    for size in sizes {
        for instance in 0..instances {
            let seed = base_seed
                ^ (size as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
                ^ (instance as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            corpus.push(extract_subgraph(tree, size, seed));
        }
    }
    corpus
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic, clippy::unwrap_used, clippy::expect_used)]

    use super::{build_corpus, extract_subgraph};
    use crate::generate::{SyntheticTree, TreeSpec};
    use std::collections::HashSet;

    fn tree() -> SyntheticTree {
        SyntheticTree::generate(TreeSpec::sized_for(4_000, 21))
    }

    #[test]
    fn extraction_is_deterministic() {
        let t = tree();
        let a = extract_subgraph(&t, 150, 99);
        let b = extract_subgraph(&t, 150, 99);
        assert_eq!(a.people, b.people);
        assert_eq!(a.families, b.families);
        assert_eq!(a.seed_person, b.seed_person);
    }

    #[test]
    fn different_seeds_start_elsewhere() {
        let t = tree();
        let starts: HashSet<_> = (0..40)
            .map(|s| extract_subgraph(&t, 60, s).seed_person)
            .collect();
        assert!(starts.len() > 20, "only {} distinct starts", starts.len());
    }

    #[test]
    fn requested_size_is_respected() {
        let t = tree();
        for size in [1_usize, 2, 17, 150, 900] {
            let g = extract_subgraph(&t, size, 5);
            assert!(g.len() <= size, "asked {size}, got {}", g.len());
            // The tree is one large component, so a modest request is always satisfiable.
            if size <= 900 {
                assert_eq!(g.len(), size, "asked {size}, got {}", g.len());
            }
        }
    }

    #[test]
    fn indices_are_dense_and_consistent() {
        let t = tree();
        let g = extract_subgraph(&t, 200, 3);
        assert_eq!(g.index_of.len(), g.people.len());
        for (i, &pid) in g.people.iter().enumerate() {
            assert_eq!(g.index_of[&pid], i as u32);
            assert!(g.contains(pid));
        }
    }

    #[test]
    fn the_subgraph_is_connected() {
        let t = tree();
        let g = extract_subgraph(&t, 250, 11);
        let selected: HashSet<_> = g.people.iter().copied().collect();

        // Re-walk from the seed inside the selection; everything must be reachable.
        let mut seen = HashSet::new();
        let mut stack = vec![g.seed_person];
        let mut buf = Vec::new();
        while let Some(p) = stack.pop() {
            if !seen.insert(p) {
                continue;
            }
            buf.clear();
            super::undirected_neighbours(&t, p, &mut buf);
            for &n in &buf {
                if selected.contains(&n) && !seen.contains(&n) {
                    stack.push(n);
                }
            }
        }
        assert_eq!(seen.len(), g.len(), "subgraph is not connected");
    }

    #[test]
    fn retained_families_carry_a_real_relationship() {
        let t = tree();
        let g = extract_subgraph(&t, 300, 13);
        for &fid in &g.families {
            let f = t.family(fid);
            let present = f
                .husband
                .iter()
                .chain(f.wife.iter())
                .chain(f.children.iter())
                .filter(|id| g.contains(**id))
                .count();
            assert!(present >= 2, "family {fid:?} contributes no edge");
        }
    }

    #[test]
    fn undirected_traversal_reaches_parents_as_well_as_children() {
        // The whole point of traversing undirected: a directed walk from a founder could only
        // ever go downward, and would never sample an ancestor-side structure.
        let t = tree();
        let g = extract_subgraph(&t, 400, 17);
        let generations: HashSet<u16> = g.people.iter().map(|&p| t.person(p).generation).collect();
        assert!(
            generations.len() >= 3,
            "sample spans only {} generation(s); the walk is behaving directionally",
            generations.len()
        );
    }

    #[test]
    fn empty_and_degenerate_requests_are_handled() {
        let t = tree();
        let g = extract_subgraph(&t, 0, 1);
        assert!(g.is_empty());
        assert!(g.families.is_empty());
    }

    #[test]
    fn corpus_covers_every_size_and_instance() {
        let t = tree();
        let corpus = build_corpus(&t, [10_usize, 20, 30], 5, 42);
        assert_eq!(corpus.len(), 15);
        for (i, g) in corpus.iter().enumerate() {
            let expected = [10, 20, 30][i / 5];
            assert_eq!(g.len(), expected);
        }
        // Instances of the same size must not be identical.
        let first_five: HashSet<_> = corpus[..5].iter().map(|g| g.seed_person).collect();
        assert!(first_five.len() > 1, "all instances started from one node");
    }
}
