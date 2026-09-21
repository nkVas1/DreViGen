# Changelog

All notable changes to DreViGen are recorded here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versioning follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html); the public API
contract begins at 1.0.

## [Unreleased]

### Added — 2026-09-21 · project foundation

- Repository established under AGPL-3.0, with editor and Git conventions.
- **Product vision** — the problem, three personas, ten principles, explicit non-goals, and the
  measurable definition of success.
- **R&D dossier** — eight research notes with a full bibliography and access dates:
  - competitive landscape (MyHeritage, MobileFamilyTree 11, Gramps, webtrees, Ancestris,
    WikiTree, and the current HCI literature);
  - genealogical data standards (GEDCOM 7.0.18, object model, dates and calendars, names and
    phonetic matching, the Genealogical Proof Standard, privacy of living people);
  - layout and rendering at 50 000 nodes;
  - synchronisation, merging and attribution;
  - designing for older adults, against WCAG 2.2 and COGA;
  - the state of visual and motion design in 2026;
  - automation and machine learning for archival work.
- **Architecture** — system overview, stack rationale, repository layout, localisation and
  security posture, testing strategy, and the five open questions for Phase 0.
- **ADRs 0001–0005** — a shared Rust core on every platform; React chosen for React Aria;
  review between identities with auto-merge within one; an own layout engine; the
  assertion/conclusion split.
- **Art direction** — *Herbarium Vivum*: the five-layer visual system, OKLCH palette with APCA
  validation, the type system, five semantic-zoom symbol grades, the cartographic apparatus,
  eight display modes, the motion specification, and the ethics of representation.
- **Asset production brief** — sketches, prompts and delivery specifications for every image
  the project needs, plus what must *not* be generated and where public-domain plates beat
  generation.
- **Roadmap** — seven phases to 1.0, each with deliverables, exit criteria and a persona
  session, plus the risk register.
- Community health files: contributing guide, security policy, code of conduct.
- Continuous integration for documentation quality.

### Added — 2026-09-21 · Phase 0 begins

- **Cargo workspace** on a pinned 1.96.1 toolchain, with the lint policy the guidelines require:
  `unsafe` forbidden, missing docs warned, clippy `all=deny` plus pedantic, `unwrap` and `panic`
  warned outside tests.
- **`drevigen-testkit`** — deterministic synthetic genealogies with the shape of real ones:
  generational layers, pedigree collapse from cousin marriage, spouses who married in from
  nowhere, remarriage, and period-accurate infant mortality. Russian names throughout, because
  irregular patronymics and sex-inflected surnames are exactly what the product must handle.
  200 000 people generate in 52 ms.
- **Undirected-BFS subgraph sampling** for layout benchmarks, following the methodology in
  Racine (2025): sampling the directed graph would yield artificially tree-like cases.
- **Spike S1** — Loro 1.16.2 against Automerge 0.12.0 at 50 000 people and 350 000 operations,
  with a counting allocator separating allocation churn from retained memory.
- **Rust CI** — format, clippy at deny-warnings, tests, a rustdoc build that fails on warnings,
  and cargo-deny for licences, bans, sources and advisories.

### Changed — 2026-09-21

- **Layout research rewritten** against McGuffin & Balakrishnan (InfoVis 2005) and Racine
  (TU Wien, 2025), both now read in full rather than cited from abstracts. Three consequences:
  exponential crowding is a *proof* that no large genealogy can be drawn in full with
  generations aligned, so the z0 overview must not be a scaled-down layout; the **dual-tree** is
  promoted to the default Workshop view; and cycles must be *classified*, not merely detected.
- **Asset brief rebuilt** as an illustrated fourteen-plate production package with engraved
  sketches, copy-ready prompts and delivery specs.
- Archivo, named in the art direction as the data face, turns out to have **no Cyrillic**.
  Golos Text substituted pending the Phase 1 type audit.

### Decided — 2026-09-21

- [**ADR 0006**](docs/02-architecture/adr/0006-loro-as-crdt-substrate.md) — Loro is the CRDT
  substrate. It builds 15.7× faster, loads 52× faster, merges 26× faster and retains 17 % less
  memory than Automerge; Automerge wins only on encoded size, 0.8 MiB against 6.6 MiB, which the
  ADR mitigates and states openly. Conditional on an on-device run, tracked as spike S1b.

[Unreleased]: https://github.com/nkVas1/DreViGen/commits/main
