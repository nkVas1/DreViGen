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

[Unreleased]: https://github.com/nkVas1/DreViGen/commits/main
