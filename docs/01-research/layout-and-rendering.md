# Layout & Rendering at Scale

*Access date: 2026-09-21. Target: a 50 000-person tree, panned and zoomed at a sustained 60 fps.*

## 1. A family tree is not a tree

The first correction any genealogy renderer must make: the structure is a **directed acyclic
graph**, not a tree.

- **Pedigree collapse.** Cousins marry. An ancestor then appears through two distinct lines, so
  the ancestor graph has diamonds in it. In endogamous or village populations this is the norm,
  not an edge case.
- **Multiple partnerships.** A person belongs to several family units, each with its own children.
- **Bidirectionality.** Ancestors go up, descendants go down, and the interesting views want both
  at once around a focal person.
- **Non-biological relationships.** Adoption, fostering and step-relationships create edges that
  are real socially and absent genetically. A view must be able to include or exclude them.

Marik (*Efficient Genealogical Graph Layout*, Springer 2016) frames the resulting problem
precisely: unconstrained planar tree layout is easy, but **layout with constraints on node ranks
and on ordering within ranks is a hard combinatorial problem** — and that is exactly what a
genealogical graph is. The same work observes that classical ancestor, descendant, hourglass and
fan charts are good for assessing one person's direct lines and **bad at showing several
interlinked families at once** — which is the view a researcher actually needs. **[established]**

## 2. Layout constraints specific to genealogy

The layout engine must honour, in roughly this priority order:

1. **Generation = layer.** People of the same generation share a horizontal band. This is the
   constraint that makes a genealogy diagram readable at a glance, and generation must be
   *computed* (longest-path from a root set) rather than taken from birth dates, which lie.
2. **Partners adjacent.** Spouses sit next to each other, with the family node between them.
3. **Children ordered and centred.** Siblings ordered by birth date, the sibling group centred
   under its family node.
4. **Crossing minimisation** within layers. NP-hard in general; the standard Sugiyama pipeline
   uses layer-by-layer barycentre/median heuristics with iterative refinement.
5. **Compactness** — horizontal extent minimised subject to the above.
6. **Stability** — see §4. This one is invisible until you violate it, and then it ruins the
   product.

The base pipeline is **Sugiyama** (Sugiyama, Tagawa & Toda, *Methods for Visual Understanding of
Hierarchical System Structures*, IEEE SMC, 1981): cycle removal → layer assignment → crossing
reduction → coordinate assignment. Genealogy modifies each stage: cycle removal is unnecessary
(the graph is acyclic by construction, barring data errors we should detect and report), layer
assignment is generation assignment, crossing reduction must treat a couple as an atomic unit,
and coordinate assignment must centre sibling groups. **[established]**

## 3. Candidate engines

| Option | Verdict |
|---|---|
| **elkjs** 0.12.0 | The most capable layered engine in JS; ELK's flagship layer-based algorithm suits directed node-link diagrams with ports. But it is ~500 KB of transpiled Java and single-threaded. |
| **d3-dag** 1.2.2 | TypeScript-first, far smaller, multiple layering and coordinate strategies including ILP-optimal crossing minimisation. Documented to freeze the browser above roughly 500 nodes / 1 500 edges. |
| **dagre** | Effectively unmaintained; superseded by the above. |
| **Graphviz (dot)** | Excellent quality, and Marik's constraint technique maps partly onto DOT directives — but it is a batch tool, not an interactive one. |
| **Own engine, Rust → WASM** | **Chosen.** |

**[judgement]** None of the JS options survives 50 000 nodes interactively, and none of them
models a couple as an atomic unit or offers layout stability across edits. We write the layout
engine in Rust, compile to WASM for the web and link it natively under Tauri, and run it off the
main thread. This is a well-defined, well-understood algorithm with a clear specification — the
right kind of thing to own. The Graphviz/ELK output serves as a quality reference in tests.

## 4. Stability: the requirement nobody documents

When a user adds one person, the rest of the diagram **must not move**. Re-running a global
layout produces a correct diagram and destroys the user's spatial memory — which for an older
user is not an inconvenience but a total loss of orientation.

Our approach:

- **Anchored incremental layout.** Existing nodes carry their assigned order within a layer as a
  soft constraint. A new node is inserted at the locally optimal position; only the affected
  sibling group and its immediate neighbourhood are re-solved.
- **Global re-layout is an explicit, named user action** ("Перестроить схему"), animated as a
  single coherent transition so the eye can follow.
- **Pinning.** A user may pin a person to a position; layout treats pins as hard constraints.

## 5. Rendering

### The 60 fps budget

At 50 000 nodes, DOM and SVG are both out — each node would need several elements, and layout
and paint alone exceed the frame budget by an order of magnitude. Canvas-based GPU rendering is
the only option. Measured community experience puts naive canvas graph rendering in trouble
somewhere above a thousand nodes, so the technique matters as much as the API. **[current]**

### API choice

**PixiJS 8.21.0** exposes both a WebGL and a WebGPU renderer. As of the access date, the Pixi
documentation states the WebGPU renderer is feature-complete but that **browser implementation
inconsistencies may cause unexpected behaviour, and WebGL is recommended for production**.
**[current]**

**[judgement]** We therefore build on a thin internal renderer abstraction, ship **WebGL 2** as
the default path and **WebGPU behind a capability check** with automatic fallback. When the
WebGPU story settles the switch is a configuration change, not a rewrite. We do not adopt Pixi's
scene graph wholesale — a general scene graph is the wrong shape for a chart where almost every
object is one of six symbol types. We use Pixi as a rendering substrate and keep our own
scene representation.

### Techniques required

- **Instanced rendering.** Every person at a given LOD is the same quad with different
  attributes. One draw call for tens of thousands of people.
- **MSDF text atlases.** Text is the real bottleneck. Multi-channel signed distance field glyph
  atlases give crisp labels at any zoom from a single texture, with Cyrillic, Latin and Greek
  ranges pre-baked and rarer glyphs rasterised on demand.
- **Viewport culling via a spatial index.** An R-tree over node bounding boxes, rebuilt
  incrementally on layout change, serving both culling and hit-testing.
- **Edge batching.** Connection lines rendered as a single instanced geometry buffer; orthogonal
  routing computed in the layout pass, not per frame.
- **Tile caching for distant LODs.** At the furthest zoom the diagram becomes a static texture
  pyramid — genuinely map-like, and effectively free to pan.
- **Off-main-thread everything.** Layout, spatial index construction and query execution live in
  a worker; the main thread only issues draw calls and handles input.

## 6. Semantic zoom — the cartographic model

Directly from McGuffin & Balakrishnan's observation (InfoVis 2005) that genealogical graphs need
representations beyond the classical chart, and from ordinary map practice: **zoom should change
the symbol, not merely its size.**

| Zoom band | Symbol | Visible information |
|---|---|---|
| **z0 — Территория** | 1–3 px dot, colour by branch | Shape and density of the whole tree; generation bands; a minimap that is the only navigation |
| **z1 — Ветвь** | Small glyph, surname clusters labelled | Which families exist and how they connect; life-span bars |
| **z2 — Семья** | Compact card: name, years | The working level for a researcher scanning a lineage |
| **z3 — Персона** | Full card: portrait, name, dates, places, source indicator | The level where editing happens |
| **z4 — Досье** | Card expands in place into the full record | Everything, without ever leaving the canvas |

Transitions between bands are cross-faded and **never reflow the layout** — positions are
scale-invariant, only symbology changes. This is what makes it feel like a map rather than a
zooming diagram.

Supporting cartographic furniture, all of it load-bearing rather than decorative:

- **Minimap** with the current viewport rectangle, always present.
- **Scale indicator** reading in generations, not pixels ("1 : 8 поколений").
- **Legend** explaining every symbol and colour currently on screen — directly answering the
  research finding that older users struggle with unlabelled iconography.
- **Landmarks** — named places the user can jump to: "я", "самый дальний предок", bookmarked
  people, the person edited last.
- **Compass / orient control** that returns to a known state. There must always be one visible
  control that means "take me back to somewhere I recognise".

## 7. The alternative views

Semantic zoom governs the main canvas. Alongside it, and sharing the same layout core:

- **Fan chart** — a sunburst cut in half, rings as generations, radiating from a focal person.
  The best-known genealogical form and the best print artefact.
- **Hourglass** — ancestors above, descendants below, focal person centred.
- **Kinship / relationship path** — how any two people are related, as a highlighted path.
- **Genogram** — medical/behavioural annotation conventions over the family structure.
- **Timeline / стратиграфия** — see below.
- **Sankey flows** — for migration between places and for descendant volume across generations.
  Genuinely the right form for "where did this family go", and misapplied anywhere else.
- **Geographic** — events on a map, with the place hierarchy and historical boundaries.

### Стратиграфия (the history-strata mode)

A dedicated display mode, not a decoration. The vertical axis is absolute time; each person is a
**bar spanning their lifetime**, not a point. Underneath the personal bars runs a **stratum of
historical events** — wars, famines, resettlements, administrative reorganisations, regionally
scoped — so it is immediately visible what each ancestor lived through, and which archival record
sets should therefore exist for them.

**[judgement]** This is both an emotional feature and a research feature: "he was 19 in 1914"
is a hypothesis generator. The historical strata dataset is per-region, versioned, community
editable, and shipped as data rather than compiled in.

## 8. Summary of obligations

| Requirement | Decision |
|---|---|
| Layout engine | Own, in Rust → WASM; Sugiyama-derived with genealogical constraints |
| Layout stability | Anchored incremental layout; global re-layout only on explicit request |
| Rendering API | Internal abstraction; WebGL 2 default, WebGPU behind capability detection |
| Node rendering | Instanced quads, one draw call per LOD class |
| Text | MSDF atlas, Cyrillic + Latin + Greek pre-baked |
| Culling & hit-testing | Incremental R-tree in a worker |
| Interaction model | Semantic zoom across five bands with map furniture |
| Alternative views | Fan, hourglass, kinship, genogram, strata timeline, Sankey, geographic |
