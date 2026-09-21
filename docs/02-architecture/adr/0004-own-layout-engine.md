# ADR 0004 — Own layout engine rather than elkjs or d3-dag

**Status:** Accepted · 2026-09-21

## Context

The canvas must lay out up to 50 000 people at interactive speed, honouring constraints that are
specific to genealogy: generation as layer, partners adjacent with the family node between them,
sibling groups ordered by birth and centred, and — critically — **stability**, so that adding one
person does not move everyone else.

Surveyed options: **elkjs 0.12.0** (the most capable layered engine in JS, roughly 500 KB of
transpiled Java, single-threaded); **d3-dag 1.2.2** (TypeScript-first, smaller, multiple
strategies including ILP-optimal crossing minimisation, documented to freeze the browser above
roughly 500 nodes and 1 500 edges); **dagre** (effectively unmaintained); **Graphviz** (excellent
quality, batch-oriented, and the genealogical constraint technique published by Marik maps only
partly onto DOT directives).

## Decision

Implement `drevigen-layout` in Rust: a Sugiyama-derived pipeline with genealogical constraints,
compiled to WASM for the web and linked natively under Tauri, running off the main thread.

## Consequences

**Gained.** Performance at the target scale. A couple modelled as an atomic ordering unit, which
no surveyed engine offers. **Anchored incremental layout** — the stability property, which none
of them offers either, and whose absence would disorient our primary persona. Deterministic
output, so layouts are reproducible in tests and in print.

**Given up.** We own a non-trivial algorithm, including crossing-reduction heuristics and
coordinate assignment. Estimated three to four weeks for a correct first version.

**Risk control.** Graphviz and ELK outputs serve as quality references in the test suite: our
crossing counts and total edge length must fall within a defined margin of theirs on shared
fixtures.

## Alternatives considered

- **elkjs in a worker.** Solves threading, not scale, not the couple-as-unit constraint, and not
  stability. Adds 500 KB.
- **d3-dag with our own pre- and post-passes.** Its documented limit is an order of magnitude
  below our target.
- **Force-directed layout.** Produces organic pictures and destroys generational reading, which
  is the entire point of a genealogical chart.
