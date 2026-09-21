# Layout & Rendering at Scale

*Access date: 2026-09-21. Revised 2026-09-21 against two primary sources read in full — see §2.*
*Target: a 50 000-person tree, panned and zoomed at a sustained 60 fps.*

## 1. A family tree is not a tree

The first correction any genealogy renderer must make: the structure is a **directed acyclic
graph**, not a tree.

- **Pedigree collapse.** Cousins marry. An ancestor then appears through two distinct lines, so
  the ancestor graph has diamonds in it.
- **Multiple partnerships.** A person belongs to several family units, each with its own children.
- **Multiple roots.** A real dataset has many founders — people whose parents are simply not
  recorded. There is no single origin to lay out from.
- **Bidirectionality.** Ancestors go up, descendants go down, and the interesting views want both
  at once around a focal person.
- **Non-biological relationships.** Adoption, fostering and step-relationships create edges that
  are real socially and absent genetically.

## 2. The two primary sources

Both were read in full rather than through summaries. Copies are in the local library; see
[library.md](./library.md).

- **McGuffin & Balakrishnan**, *Interactive Visualization of Genealogical Graphs*, IEEE InfoVis
  2005. The graph-theoretic analysis of why these graphs are hard, and the **dual-tree**
  construction.
- **Racine**, *Efficient Algorithms for Drawing Large Genealogy Trees*, MSc thesis, TU Wien,
  September 2025, supervised by Di Bartolomeo and Nöllenburg. Adapts Sugiyama, force-directed
  and clustering approaches to genealogy and benchmarks them against a 30 000-person Habsburg
  dataset.

### 2.1 Exponential crowding — the result that settles the architecture

McGuffin & Balakrishnan analyse an idealised genealogical graph `G*` in which every node has two
parents, one sibling, one spouse, and every marriage produces one child of each sex. Draw it with
uniform node sizes and each generation on one row, and the following holds. **[established]**

> Each node has 1 sibling, 4 first cousins, 16 second cousins, and **4ⁿ nth cousins** — and all of
> them belong to the *same generation*, so all of them must fit in a single row, whose available
> space grows only **linearly** with distance from the centre. The consequence is that the
> **edge-length-to-node-size ratio becomes arbitrarily high**.

The authors note this is *qualitatively worse* than Munzner's observation about embedding trees in
Euclidean space, because there the crowding worsens with depth, whereas here **the exponential
crowding occurs within every single generation** as more of the graph is displayed.

**This is a proof, not an opinion, and it has one architectural consequence:** there is no layout
algorithm — ours or anyone's — that can draw a large genealogy in full with generations aligned
and stay readable. Their own conclusion: *"A better solution may be to display subgraphs that are
automatically laid out, and allow the user to flexibly transition between subgraphs."*

Racine reaches the same place empirically. The adapted Sugiyama layout of the full 30 000-person
Habsburg dataset renders correctly and "becomes visually dense at this scale" — the algorithm
succeeds and the picture is still unusable.

**Obligation.** Our semantic-zoom design is vindicated, but with a sharper requirement than we had
written: **the z0 overview must not be a scaled-down full layout.** It has to be a genuinely
different representation — density, territory, branch mass — because a correct full layout is
unreadable no matter how far you zoom out. Whole-graph layout is for *navigation and overview*;
readable genealogical reading happens in subsets.

### 2.2 Two kinds of intermarriage, and why the distinction is load-bearing

McGuffin & Balakrishnan separate them, and the separation matters for the layout engine:
**[established]**

| | Definition | Graph consequence |
|---|---|---|
| **Type 1** | Spouses who are consanguine — cousins, however distant | A **diamond**: two distinct directed paths between the same pair of nodes |
| **Type 2** | Spouses related only *conjugally*, through some other marriage — two sisters marrying two brothers, or a woman marrying her late husband's brother | An undirected cycle, but **no diamond** |

A genealogical graph free of type 1 intermarriage is a **multitree** — a diamond-free DAG in which
every node `x` has a well-defined tree of ancestors `A(x)` and tree of descendants `D(x)`. Type 2
intermarriage is harmless to this property. **Only type 1 breaks it.**

Racine catalogues the undirected cycles that arise in practice and makes the complementary point
that most of them are perfectly ordinary: two parents with two shared children already form an
undirected cycle. Directed cycles, by contrast, are biologically impossible and should be treated
as data errors.

**Pedigree collapse is guaranteed, not incidental.** Without type 1 intermarriage a person would
have 2ⁿ ancestors n generations back, which at 30 years per generation exceeds the carrying
capacity of the Earth in under 2 000 years. Every real ancestry contains it, if traced far enough.
In practice many datasets are *locally* free of it.

**Obligation.** The preprocessing pass classifies cycles rather than merely detecting them.
McGuffin & Balakrishnan give a directly implementable method: BFS the underlying undirected graph
to find cycles, then count how many times edge direction changes around each cycle —

- **0 changes** → a directed cycle → a **data error**, surfaced to the user, never silently fixed;
- **2 changes** → a **diamond**, i.e. type 1 intermarriage;
- **4 or more** → type 2, permitted in a multitree and requiring nothing.

Where a diamond must be broken for a tree-shaped view, one edge is marked as skipped **and drawn
in a distinct colour** so the user can see that the picture is hiding a real relationship. We
extend this: in DreViGen a broken diamond is a first-class annotation with its own legend entry,
because concealing consanguinity from a genealogist is a correctness failure, not a simplification.

### 2.3 The dual-tree

The central contribution of McGuffin & Balakrishnan, and the strongest single idea either paper
offers. **[established]**

An **hourglass chart** is `A(x) ∪ D(x)` — one person's ancestors above and descendants below.
Choosing `x` is a forced trade: pick someone older and you see more descendants but fewer
ancestors.

A **dual-tree** is `A(x) ∪ D(y)` where `y` is an ancestor of `x` — the roots are *offset*. Because
`A(x) ⊃ A(y)` and `D(y) ⊃ D(x)`, it contains a strict superset of the hourglass information, and
`x` and `y` can be chosen at the most recent and oldest generations to maximise coverage.

```text
   hourglass  A(x) ∪ D(x)              dual-tree  A(x) ∪ D(y)
                                          y ──────┐
        ╲   ╱                              ╲      │  ← axis: the path
         ╲ ╱                                ╲     │    between the two roots
          x                                  ╲    │
         ╱ ╲                                  x ──┴────
        ╱   ╲                                ╱  ╲   ╲
                                            ╱    ╲   ╲
   one trade-off: ancestors vs          every ancestor of x, and every
   descendants                          descendant of y, at once
```

Its properties are exactly the ones the crowding result says are otherwise unobtainable:

- **no edge crossings**;
- **nodes ordered by generation**;
- **scales as well as a single tree** — the crowding inside it is no worse than inside one tree;
- it browses **any multitree**, including nodes with multiple spouses, type 2 intermarriage, and
  nodes with more than two parents.

The use case that sells it: pick `y` as the oldest paternal ancestor of `x`, and the dual-tree
shows **every ancestor of `x` together with every person sharing `x`'s surname**, in one
crossing-free generational diagram. The authors know of no other traditional, scalable depiction
that can do this — and neither do we, after surveying the field.

**Layout is linear time.** Compute preliminary embeddings `E_A` and `E_D` with Reingold–Tilford
(better: the Buchheim–Jünger–Leipert linear-time improvement), align them by generation, then
place each node `n` on the axis at the weighted average

```text
p_F = (a · p_A + d · p_D) / (a + d)
```

where `a` and `d` are `n`'s ancestor and descendant counts. The rationale is that a node with many
edges in one tree should stay near its position in that tree, so no edge acquires an extreme slope.

**Obligation — a change to our plan.** The dual-tree becomes a first-class view, added to the
catalogue in [roadmap.md](../04-planning/roadmap.md) Phase 4 and, more importantly, promoted: it is
the natural **default working view in the Workshop**, with the Atlas as the overview it hangs off.
The interaction is *change the roots and transition*, which is precisely the "display subsets and
let the user move between them" the crowding result demands.

### 2.4 Other representations worth having

- **Indented-outline dual-tree.** The same construction drawn in outline style, using the
  left-child/right-sibling edge convention from Venolia & Neustaedter. More space-efficient with
  long text labels — and Russian full names with patronymics *are* long. Its extreme aspect ratio
  is an advantage: the user scrolls in one direction only. A strong candidate for the phone layout,
  where a node-link canvas is cramped.
- **Nested containment** with the user's current focus as a temporary root — the treemap idea
  applied to a genealogical free tree.
- **Fractal / recursive subdivision** (both papers, independently). Bounded edge length and no
  crossings, paid for by giving up generational ordering.
- **Bertin's line-segment notation**, from *Sémiologie graphique*: each individual is a line
  segment, thick for men and thin for women, and each nuclear family is a point. Elegant, and
  McGuffin & Balakrishnan note it suffers the same exponential crowding. Worth knowing as the
  ancestor of our strata mode, where a person is also a segment.

### 2.5 What the benchmarks actually say

Racine's evaluation is the only rigorous published comparison we found. Method: 60 000 subgraphs
(200 sizes × 300 instances) extracted from the 30 000-person Habsburg dataset, laid out with OGDF
on a Ryzen 5 2500U. Node size fixed at 30×30 px so that shrinking the drawing cannot game the area
metric. **[established]**

| Approach | Area & edge length | Crossings | Runtime |
|---|---|---|---|
| **Sugiyama, default** | efficient spacing, moderate | moderate | **highly efficient** |
| **Sugiyama, genealogy-adapted** | efficient spacing, moderate | moderate | slightly slower than default |
| Force-directed, default | most compact | **fewest** | slower than Sugiyama |
| Force-directed, genealogy-adapted | largest, longest edges | more | slowest |
| Force-radial, genealogy-adapted | moderate | more than default | slow |

Three conclusions we take directly:

1. **Sugiyama is the right base**, and the genealogy-specific constraints cost very little runtime
   on top of it. This confirms [ADR 0004](../02-architecture/adr/0004-own-layout-engine.md).
2. **Force-directed layouts are a trap.** They win the aesthetic metrics precisely by ignoring
   generational structure — which is the entire content of a genealogical chart. Racine states the
   trade openly: imposing chronological layering *necessarily* worsens area, edge length and
   crossings, and that cost "must be interpreted as the result of incorporating meaningful
   domain-specific information rather than as a flaw in the algorithm." **We do not optimise for
   crossing counts at the expense of generational reading, and we will not report those metrics
   without saying so.**
3. **The scale gap is enormous.** Constrained layouts are already consuming hundreds of
   milliseconds at **200 nodes** on that hardware. We need 50 000 at interactive rates. That gap
   cannot be closed by a faster full-graph layout; it is closed by incremental layout, level of
   detail, and subset views. Which is what the crowding result said in theory.

### 2.6 Benchmark methodology we adopt

One methodological detail from Racine is worth copying exactly: subgraph samples are extracted by
**BFS over the undirected graph**, never the directed one. BFS on the directed graph yields
artificially tree-like samples — fewer cycles, more single-parent nodes — and would flatter any
algorithm under test. **[established]**

**Obligation.** `drevigen-testkit` gains an undirected-BFS subgraph extractor, so our benchmark
corpus has the same structural honesty. Node size is fixed for area metrics. Where possible we
report against the same metric set: runtime, crossings, min/mean/max edge length, bounding-box
area.

### 2.7 The fractal method and its exponential trap

Racine's novel contribution: each person is a rectangle that recursively subdivides its area among
its descendants, alternating horizontal and vertical splits per generation, with a weight function
allocating space. The weight choice matters — `w(v) = |nodesUnder(v)|` preserves the full tree
while `w(v) = depth(v) + 1` leaves parts of it invisible. Runtime is linear to n = 500
(~400 µs). **[established]**

**But:** the method draws a subtree once per distinct ancestral path, so its cost depends on the
number of unique paths, **which grows exponentially with consanguinity cycles**. On a heavily
interconnected genealogy it blows up. Racine flags this as an open scalability limitation and
proposes two remedies: placeholder nodes with interactive expansion, or arrow references to
already-drawn subtrees (which introduces a fresh crossing-minimisation problem).

**[judgement]** The fractal view is worth having as an overview at z0, where it does something no
node-link layout can — show every person at once with global structure intact. We adopt the
placeholder remedy: a repeated subtree is drawn once and referenced, because unbounded duplication
is also *wrong*, not merely slow. It shows one person as many people.

### 2.8 Clustering is an interpretive act

Racine implements birthplace clustering with a size threshold and a co-parent edge threshold, and
then says the thing that most papers leave out: **clustering "is not a neutral technique but an
interpretive act"**. Birthplace clustering privileges geographic identity; clustering by lineage
continuity or social rank would tell a different historical story from the same data.
**[established]**

**Obligation.** If DreViGen clusters, the criterion is **always visible on screen and always
switchable**, never a silent default. A view that silently groups a user's ancestors by birthplace
is making a historical argument on their behalf.

### 2.9 The user finding we should not ignore

McGuffin & Balakrishnan's prototype was shown to one practising genealogist, who found it "very
clear" and "very easy" and was "impressed with its manoeuvrability" — and said the unfamiliarity
"takes getting used to", and **specifically missed a symbol explicitly linking spouses**, which
conventional charts have. **[current]** — one participant, so weak evidence, but it points at a
real thing.

**Obligation.** Our specimen design must show marriage explicitly rather than by adjacency alone.
For our primary persona this is not a preference but a requirement: adjacency is a convention he
would have to be taught, and a drawn link is not.

## 3. Layout constraints specific to genealogy

Both sources agree on the constraint set, in roughly this priority order:

1. **Generation = layer**, and generation must be **detected**, not taken from topological depth
   and not from birth years. Racine is explicit: *"Rather than computing layers solely from
   topological depth, generations should be explicitly detected and assigned to layers."*
2. **Partners share a layer**, adjacent, with the family node between them and centred.
3. **Sibling groups contiguous and ordered** by birth, centred under their family node.
4. **A couple is an atomic ordering unit** during crossing reduction — Racine: *"parents linked
   through a marriage should be treated as a unit during ordering to prevent unnecessary crossings
   between their children's edges."*
5. **Crossing minimisation** within layers: NP-hard, so barycentre/median heuristics with
   iterative refinement.
6. **Fixed layer anchors** — key ancestors pinned to a layer for chronological consistency.
7. **Compactness**, subject to all of the above.
8. **Stability** — see §5. Invisible until violated, and then it ruins the product.

The base pipeline is **Sugiyama** (Sugiyama, Tagawa & Toda, IEEE SMC, 1981): cycle removal → layer
assignment → crossing reduction → coordinate assignment. Cycle removal is unnecessary for us
beyond error detection, since directed cycles are biologically impossible.

## 4. Candidate engines

| Option | Verdict |
|---|---|
| **elkjs** 0.12.0 | The most capable layered engine in JS; ~500 KB of transpiled Java, single-threaded. |
| **d3-dag** 1.2.2 | TypeScript-first, smaller, multiple layering and coordinate strategies including ILP-optimal crossing minimisation. Documented to freeze the browser above roughly 500 nodes / 1 500 edges. |
| **OGDF** (C++) | What Racine benchmarked. Excellent quality; its numbers above define the state of the art — and show constrained layout already costing hundreds of ms at 200 nodes. |
| **dagre** | Effectively unmaintained. |
| **Graphviz** | Excellent quality, batch-oriented. |
| **Own engine, Rust → WASM** | **Chosen.** See [ADR 0004](../02-architecture/adr/0004-own-layout-engine.md). |

## 5. Stability: the requirement nobody documents

When a user adds one person, the rest of the diagram **must not move**. Re-running a global layout
produces a correct diagram and destroys the user's spatial memory — which for an older user is not
an inconvenience but a total loss of orientation.

Neither paper addresses this, because both target static drawing. It is ours to solve.

- **Anchored incremental layout.** Existing nodes carry their assigned order within a layer as a
  soft constraint. A new node is inserted at the locally optimal position; only the affected
  sibling group and its immediate neighbourhood are re-solved.
- **Global re-layout is an explicit, named user action** ("Перестроить схему"), animated as a
  single coherent transition so the eye can follow.
- **Pinning.** A user may pin a person to a position; layout treats pins as hard constraints —
  the interactive form of Racine's fixed layer anchors.

## 6. Rendering

### 6.1 The 60 fps budget

At 50 000 nodes, DOM and SVG are both out. Canvas-based GPU rendering is the only option, and the
technique matters as much as the API.

### 6.2 API choice

**PixiJS 8.21.0** exposes both a WebGL and a WebGPU renderer. As of the access date, the Pixi
documentation states the WebGPU renderer is feature-complete but that **browser implementation
inconsistencies may cause unexpected behaviour, and WebGL is recommended for production**.
**[current]**

**[judgement]** We build on a thin internal renderer abstraction, ship **WebGL 2** as the default
path and **WebGPU behind a capability check** with automatic fallback. We use Pixi as a rendering
substrate and keep our own scene representation — a general scene graph is the wrong shape for a
chart where almost every object is one of six symbol types.

### 6.3 Techniques required

- **Instanced rendering.** One draw call for tens of thousands of people at a given LOD.
- **MSDF text atlases.** Text is the real bottleneck; Cyrillic, Latin and Greek pre-baked, rarer
  glyphs rasterised on demand.
- **Viewport culling via a spatial index.** An incrementally maintained R-tree serving both
  culling and hit-testing.
- **Edge batching.** One instanced geometry buffer; orthogonal routing computed in the layout
  pass, not per frame.
- **Tile caching for distant LODs.** At the furthest zoom the diagram is a static texture pyramid.
- **Off-main-thread everything.** Layout, index construction and queries in a worker.

## 7. Semantic zoom — the cartographic model

**Zoom changes the symbol, not merely its size.**

| Zoom band | Symbol | Visible information |
|---|---|---|
| **z0 — Территория** | Density field / fractal overview, **not** a shrunken layout | Shape and mass of the whole tree; branch distribution; the minimap is the only navigation |
| **z1 — Ветвь** | Small glyph, surname clusters labelled | Which families exist and how they connect; life-span bars |
| **z2 — Семья** | Compact card: name, years | The working level for scanning a lineage |
| **z3 — Персона** | Full card: portrait, name, dates, places, source indicator | Where editing happens |
| **z4 — Досье** | Card expands in place into the full record | Everything, without leaving the canvas |

Transitions cross-fade and **never reflow the layout**. Supporting cartographic furniture — minimap,
generation scale, legend, landmarks, compass-home — is specified in
[art-direction.md](../03-design/art-direction.md) §7.

## 8. The view catalogue

Semantic zoom governs the main canvas. Alongside it, sharing the same layout core:

- **Двойное древо (dual-tree)** — `A(x) ∪ D(y)`, node-link and indented-outline forms. **Promoted
  to the default Workshop view** on the evidence in §2.3.
- **Веер (fan chart)** — a sunburst cut in half; the best print artefact.
- **Песочные часы (hourglass)** — the dual-tree's degenerate case, kept because it is familiar.
- **Родство (kinship path)** — how any two people are related.
- **Генограмма** — medical and behavioural annotation conventions.
- **Стратиграфия** — time vertical, lives as bars, historical stratum beneath.
- **Потоки (Sankey)** — migration between places; descendant volume by generation.
- **География** — events on a map with period-correct boundaries.

## 9. Summary of obligations

| Requirement | Decision |
|---|---|
| Whole-graph readable layout | **Impossible by proof** (§2.1). Subsets plus transitions, not a better global layout. |
| z0 overview | A different representation — density or fractal — never a scaled-down full layout |
| Cycle handling | Classify by direction-change count: 0 = data error, 2 = type 1 diamond, ≥4 = type 2, harmless |
| Broken diamonds | Drawn distinctly, with a legend entry. Never silently hidden. |
| Dual-tree | First-class view, default in the Workshop; Reingold–Tilford per tree, weighted-average axis merge, linear time |
| Indented-outline dual-tree | The phone layout candidate; handles long Russian names without extra whitespace |
| Layout engine | Own, in Rust → WASM; Sugiyama-derived, genealogy-constrained |
| Generation assignment | Detected generations, not topological depth, not birth years |
| Couples | Atomic ordering unit during crossing reduction |
| Layout stability | Anchored incremental layout; global re-layout only on explicit request; user pins as hard constraints |
| Metrics | Report runtime, crossings, edge lengths, area at fixed node size — and state the chronological-layering trade explicitly |
| Benchmark corpus | Subgraphs extracted by BFS over the **undirected** graph |
| Fractal overview | Adopted with placeholder references; unbounded duplication is wrong, not just slow |
| Clustering | Criterion always visible and switchable; never a silent default |
| Marriage | Drawn with an explicit link symbol, not implied by adjacency |
| Rendering | WebGL 2 default, WebGPU behind capability detection; instanced quads, MSDF text, R-tree culling |
