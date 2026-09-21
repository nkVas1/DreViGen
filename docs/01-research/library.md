# Reference Library

Papers read **in full** rather than through abstracts or secondary summaries. Each entry records
what it actually contributed and where that landed in the project.

## Why this file exists and the PDFs do not

The PDFs live in `library/papers/`, which is **git-ignored**. Most are under publisher copyright
and redistributing them in a public repository would be infringement, however convenient. This
index is the part that can be shared: full citations, stable links, and — the useful part — what
each source changed.

To reproduce the library, follow the links below. Where a paper is open access the link resolves
to a free PDF; where it is not, it resolves to the publisher's page.

## Entries

### McGuffin, M. J. & Balakrishnan, R. — *Interactive Visualization of Genealogical Graphs*

IEEE Symposium on Information Visualization (InfoVis), 2005. 8 pp.
Author copy: <https://www.dgp.toronto.edu/~ravin/papers/infovis2005_geneology.pdf>
Local: `library/papers/mcguffin-2005-interactive-visualization-genealogical-graphs.pdf`
Copyright IEEE — **do not redistribute**.

**What it contributed.** The graph-theoretic foundation of the whole canvas design.

- The **exponential crowding** result: nth cousins number 4ⁿ and all belong to the *same*
  generation, so they must fit a row whose space grows only linearly. The edge-length-to-node-size
  ratio therefore becomes arbitrarily high. This is a proof that no algorithm can draw a large
  genealogy in full with generations aligned and stay readable.
- The **type 1 / type 2 intermarriage** distinction, and the cycle-classification algorithm that
  tells them apart by counting direction changes around an undirected cycle.
- **Multitrees**: a genealogy free of type 1 intermarriage is a diamond-free DAG in which every
  node has a well-defined ancestor tree and descendant tree.
- The **dual-tree** `A(x) ∪ D(y)` — crossing-free, generation-ordered, scales like a single tree,
  and strictly dominates the hourglass chart. With the weighted-average axis merge and
  Reingold–Tilford per subtree, it lays out in linear time.
- The indented-outline dual-tree, the nested-containment and fractal alternatives, and Bertin's
  line-segment notation as prior art.
- One practising genealogist's feedback, including the request for an **explicit spouse-link
  symbol**.

**Where it landed.** [`layout-and-rendering.md`](./layout-and-rendering.md) §2.1–2.4, §2.9 — and
it promoted the dual-tree to the default Workshop view.

### Racine, F. — *Efficient Algorithms for Drawing Large Genealogy Trees*

MSc thesis, TU Wien, September 2025. 102 pp. Supervised by Sara Di Bartolomeo; advisor Martin
Nöllenburg.
<https://repositum.tuwien.at/handle/20.500.12708/220457>
Local: `library/papers/racine-2025-drawing-large-genealogy-trees.pdf`
Creative Commons — check the licence statement in the PDF before reuse.

**What it contributed.** The only rigorous published benchmark of layout approaches on real
genealogical data that we found.

- Evaluation over **60 000 subgraphs** (200 sizes × 300 instances) drawn from a 30 000-person
  Habsburg dataset, using OGDF: Sugiyama is by far the most efficient and the genealogy-specific
  constraints cost it very little; force-directed layouts win the aesthetic metrics only by
  discarding generational structure.
- The methodological detail we copied: sample subgraphs by **BFS over the undirected graph**,
  because BFS over the directed graph produces artificially tree-like samples.
- The explicit statement that **chronological layering necessarily worsens area, crossings and
  edge length**, and that this is the price of meaning rather than a defect.
- Concrete Sugiyama adaptations: detected generations as layers, spouses on one layer, sibling
  groups contiguous and birth-ordered, **couples treated as an atomic unit during crossing
  reduction**, fixed layer anchors.
- A novel **fractal layout** — recursive rectangle subdivision alternating split direction per
  generation, weighted by `|nodesUnder(v)|` — with the crucial caveat that its redundancy grows
  **exponentially with consanguinity cycles**.
- Birthplace clustering, together with the honest warning that clustering is *"not a neutral
  technique but an interpretive act"*.

**Where it landed.** [`layout-and-rendering.md`](./layout-and-rendering.md) §2.5–2.8, §3 — it
confirms [ADR 0004](../02-architecture/adr/0004-own-layout-engine.md) and sets our benchmark
methodology.

## Wanted

Papers we would like to read in full. If you can supply any of these, they go straight into the
dossier — see [CONTRIBUTING.md](../../CONTRIBUTING.md).

| Work | Why we want it |
|---|---|
| Mařík, *Efficient Genealogical Graph Layout*, Springer LNCS 2016, doi:10.1007/978-3-319-50901-3_45 | The layered multitree constraint technique, read only through its abstract so far |
| Furnas & Zacks, *Multitrees: Enriching and Reusing Hierarchical Structure*, CHI 1994 | The origin of multitrees and the centrifugal view |
| Buchheim, Jünger & Leipert, *Improving Walker's Algorithm to Run in Linear Time*, GD 2002 | The tree layout we will actually implement |
| Siu et al., arXiv:2411.07869 | Read only as abstract and contributions; the full interview findings would sharpen the two-profile design |
| Elizabeth Shown Mills, *Evidence Explained*, 4th ed. 2024 | The citation templates the Workshop must implement |
| Purchase, graph-drawing aesthetics metrics (cited by Racine as [Pur97], [Pur02]) | The metric definitions our benchmarks should match |

## Reading standard

A source is cited in this project only if someone read the part being cited. An abstract is
evidence that a paper exists, not evidence for a claim. Where the dossier rests on an abstract or
a secondary summary, it says so — see the **[current]** and **[judgement]** tags in
[`README.md`](./README.md).
