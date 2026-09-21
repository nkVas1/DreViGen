# Development Roadmap

*Version 1, 2026-09-21. Revised at the end of each phase; history in git.*

---

## Ground rules

1. **Every phase ends with runnable software.** Never a skeleton, never "the UI comes later".
   If a phase cannot end with something a person can open and use, it is scoped wrongly.
2. **No placeholder code.** No `TODO: implement later`, no stub that returns fake data. A feature
   is either in the phase or it is not.
3. **Each phase ends with a persona session** — the primary persona attempts a fixed task list,
   unassisted, recorded. His failures are logged as bugs with severity, not as feedback.
4. **Each phase ends with an ownability check** — screenshots with the branding removed, archived
   to `design-archive/`.
5. **Technical debt is paid in the phase that creates it.** There is no debt backlog.
6. **Durations are working-week estimates for one developer**, and they are estimates. The order
   is the commitment; the dates are not.

## At a glance

```
  PHASE 0  Основание        ████                                  3 weeks    spikes, skeleton, CI
  PHASE 1  Ядро                 ████████████                     10 weeks    model, GEDCOM, canvas, Hall
  PHASE 2  Мастерская                       ████████              8 weeks    sources, evidence, research
  PHASE 3  Родня                                    ████████      8 weeks    server, accounts, contributions
  PHASE 4  Выражение                                    ██████    6 weeks    charts, print, export, publish
  PHASE 5  Автоматика                                     ██████  7 weeks    HTR, faces, record linkage
  PHASE 6  Огранка                                         █████  6 weeks    performance, a11y, l10n, release
                                                                 ────────
                                                                  48 weeks
```

Releases are cut at the end of every phase: `v0.1` … `v0.6`, then **`v1.0`** at the close of
Phase 6. Versioning is semver; the public API contract begins at 1.0.

---

## Phase 0 — Основание · *Foundation*

**Goal:** prove the five risky assumptions, and stand up a repository that can build and ship
from day one.

**Duration:** 3 weeks · **Release:** `v0.0` (internal)

### Spikes — each timeboxed, each ending in an ADR

| # | Question | Budget | Pass criterion |
|---|---|---|---|
| S1 | **Loro at scale.** Can a 50 000-person tree with a 100 000-operation history fork, diff and merge within budget on a mid-range Android device? | 4 days | Merge < 2 s, memory < 300 MB. Fail → Automerge 3.5, re-spike 2 days |
| S2 | **WASM core size.** Does the full core compiled to WASM stay inside the first-paint budget? | 2 days | < 2.5 MB gzipped, or a clean eager/lazy split identified |
| S3 | **MSDF atlas coverage.** How much Cyrillic + Latin + Greek can be pre-baked before atlas size dominates? | 3 days | Full Russian and German coverage < 4 MB, with a working on-demand path for the rest |
| S4 | **OPFS concurrency.** Does the single-writer model hold under our access pattern, with `SQLITE_BUSY` handled? | 2 days | 10 000 writes with no data loss, no deadlock, under simulated contention |
| S5 | **Tauri mobile capability.** Which plugins exist on both iOS and Android; what must we write ourselves? | 3 days | A written capability matrix; every gap has an owner and an estimate |

### Deliverables

- [ ] pnpm + Cargo workspace, the full directory structure from the architecture overview
- [ ] Toolchain pinned: Rust 1.96.1, Node 24.11.1, pnpm 11.10.0, exact versions throughout
- [ ] CI: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `tsc --noEmit`,
      `eslint`, `vitest`, `cargo audit`, `pnpm audit`, WASM size budget — all blocking
- [ ] Release pipeline producing signed artefacts for all six targets, even if the app is empty
- [ ] `tokens` package: the OKLCH → sRGB + APCA pipeline, with contrast failures breaking the build
- [ ] Fonts vendored and subset; the type audit from the art direction resolved
- [ ] The app opens on Windows, Android and in a browser, shows one screen, and is installable

### Exit criteria

Five ADRs written. The repository builds green on every target. A person can install the app on
a phone and see a correctly typed, correctly coloured screen.

---

## Phase 1 — Ядро · *Core*

**Goal:** a genuinely usable single-user genealogy application. Someone can build a real family
tree in it, import their existing GEDCOM, and show it to relatives.

**Duration:** 10 weeks · **Release:** `v0.1`

### Workstreams

**1.1 · Domain model** (`drevigen-core`, 2 weeks)
- [ ] Person, Family, Event with roles, Place hierarchy, Source, Citation, Repository, Note,
      Media, Tag, Attribute, Association
- [ ] Assertion and Conclusion layers (ADR 0005), with the resolved-value projection
- [ ] Domain invariants with informative errors: no cycles, no child before parent, no
      contradictory role assignment
- [ ] Property-based tests over generated family graphs

**1.2 · Dates and calendars** (`drevigen-date`, 1.5 weeks)
- [ ] GEDCOM 7 date grammar, complete
- [ ] Julian ↔ Gregorian ↔ Hebrew ↔ French Republican conversion
- [ ] Interval arithmetic and three-valued temporal predicates
- [ ] Dual-dating (`1747/8`), phrases, all approximation qualifiers
- [ ] Locale-aware formatting for ru / en / de

**1.3 · GEDCOM** (`drevigen-gedcom`, 2 weeks)
- [ ] 7.0.18 parser, serializer, validator
- [ ] Tolerant 5.5 / 5.5.1 import
- [ ] Verbatim preservation of unrepresentable structures, with a loss report shown on import
- [ ] Golden-file round-trip tests against exports from Gramps, MyHeritage, MacFamilyTree,
      Ancestris and webtrees

**1.4 · Storage** (`drevigen-store`, 1.5 weeks)
- [ ] SQLite schema, migrations, WAL
- [ ] Append-only operation log; undo and redo as log traversal
- [ ] FTS5 search index with phonetic expansion
- [ ] `.dvg` container: read and write, `.gdz`-compatible core
- [ ] OPFS path for the web build, single writer, `SQLITE_BUSY` handled

**1.5 · Layout engine** (`drevigen-layout`, 2.5 weeks)
- [ ] Generation assignment by longest path
- [ ] Ordering with couples as atomic units; barycentre crossing reduction
- [ ] Coordinate assignment with centred sibling groups
- [ ] **Anchored incremental layout** — the stability property
- [ ] Benchmarks at 1 k / 10 k / 50 k / 200 k; quality compared against Graphviz and ELK

**1.6 · Canvas** (`packages/canvas`, 2.5 weeks)
- [ ] WebGL 2 renderer with instanced symbols; WebGPU behind a capability check
- [ ] MSDF text pipeline
- [ ] Incremental R-tree for culling and hit-testing
- [ ] All five semantic-zoom bands with cross-fade and no reflow
- [ ] Spring-driven pan and zoom, interruptible, velocity-carrying

**1.7 · The Hall** (`packages/app`, 2 weeks)
- [ ] First run: create a tree, or import a GEDCOM, in under two minutes
- [ ] Person dossier opening in place on the canvas
- [ ] Add a person, add a relationship, edit, delete-as-tombstone, undo
- [ ] Photo import with the plate-well presentation
- [ ] Search with phonetic matching
- [ ] The full apparatus: home, minimap, legend, scale, landmarks
- [ ] ru / en / de complete, no hard-coded strings

### Exit criteria

- A 50 000-person synthetic tree pans and zooms at a sustained 60 fps on the reference machine.
- A real GEDCOM from each of the five reference applications imports and re-exports losslessly.
- **The persona test:** unassisted, he opens the app, finds his grandson, and views a photo.
- The ownability test passes.

---

## Phase 2 — Мастерская · *The Workshop*

**Goal:** the research apparatus. A working genealogist can run real research in DreViGen and
produce work that meets the Genealogical Proof Standard.

**Duration:** 8 weeks · **Release:** `v0.2`

- [ ] **Sources and citations** — repository / source / citation hierarchy, citation templates
      following *Evidence Explained* patterns, confidence levels, media attached to citations
- [ ] **The evidence surface** — assertions visible per fact, contradictions shown as
      contradictions, conclusions written with reasoning, the open-question state
- [ ] **Research plan** — a question, the sources to check, priority, status
- [ ] **Research log** — what was checked and when, **including negative results**; searched and
      not found is evidence and no surveyed tool records it
- [ ] **Contradiction queue** — every unresolved conflict in the tree, in one place, workable
- [ ] **Proof arguments** — compiled from assertions and conclusions into an exportable narrative
- [ ] **Validation suite** — implausible lifespans, impossible ages at parenthood, date
      inconsistencies, orphaned records, unsourced changes to sourced facts. Warnings, never
      blocks, each with an explanation and a fix
- [ ] **Places manager** — the hierarchy, coordinates, historical names with validity periods,
      external enrichment, merge and split
- [ ] **Hall / Workshop boundary** — profile switching, with the Hall provably free of Workshop
      chrome
- [ ] **Marginalia layer** — the hand-written evidence layer from the art direction

### Exit criteria

A real research task, taken from start to sourced conclusion, entirely inside the application.
A proof argument exports to PDF in a form a certified genealogist would accept.

---

## Phase 3 — Родня · *Kin*

**Goal:** the family collaborates. Server, accounts, contributions, sync — the whole loop.

**Duration:** 8 weeks · **Release:** `v0.3`

- [ ] **Server** (`apps/server`) — Axum, SQLx, PostgreSQL 18, Valkey; Docker Compose; Caddy with
      automatic certificates; one-command install on the Selectel VPS; documented backups and
      restore, with restore actually tested
- [ ] **Accounts** — passkeys first, emailed magic-link fallback, no password. Invitation by
      link, roles (owner / maintainer / contributor / viewer)
- [ ] **Sync protocol** — operation-log exchange, resumable, bandwidth-conscious, conflict-safe
- [ ] **Within-identity merge** — a user's own devices converge silently
- [ ] **Contributions** — semantic diff, change classification (clean / conflicting /
      suspicious), the review interface, comments, approve and merge, request changes
- [ ] **Conflict resolution UI** — mine / theirs / **both as competing assertions** / park as an
      open question
- [ ] **Attribution** — blame on any fact, contributor pages, honest sourcing-rate display
- [ ] **History** — the full tree history, browsable, filterable, revertible
- [ ] **Suggest a correction** — the lightweight path for a viewer who spots a wrong date
- [ ] **Privacy** — living detection, per-level policy, redaction enforced at the serialisation
      boundary, true erasure
- [ ] **Notifications** — in-app and email, batched, never noisy
- [ ] **Media sync** — content-addressed, deduplicated, resumable, bandwidth-aware on mobile

### Exit criteria

Three people on three devices, one of them offline for a week, reconcile into a correct
canonical tree with every change attributed. A restore from backup produces a byte-identical
tree.

---

## Phase 4 — Выражение · *Expression*

**Goal:** the tree becomes something you give away. Charts, books, posters, sites.

**Duration:** 6 weeks · **Release:** `v0.4`

- [ ] **The chart catalogue** — ancestor, descendant, hourglass, fan, kinship, genogram,
      statistics, timeline. This is the MobileFamilyTree bar, and it is a floor
- [ ] **Стратиграфия mode** — lives as bars over a historical stratum; the regional history
      dataset shipped as versioned data
- [ ] **Потоки (Sankey)** — migration between places, descendant volume by generation
- [ ] **География** — events on a map with period-correct boundaries
- [ ] **Print pipeline** — true vector PDF, CMYK-aware, poster sizes to A0, deckle and plate-mark
      treatment, imposition for booklets
- [ ] **Family book generator** — a narrative document with photographs, sources and an index,
      generated from the tree and editable afterwards
- [ ] **Web publish** — a static, privacy-filtered site for relatives, hostable anywhere,
      regenerated on demand
- [ ] **Sharing** — a deep link to any person or view, with the viewport in the URL
- [ ] **Slideshow** — the Hall's ambient mode: photographs with context, for a family gathering

### Exit criteria

A printed A2 fan chart that looks like it came from a press. A generated family book that a
relative would keep.

---

## Phase 5 — Автоматика · *Automation*

**Goal:** attack the labour that actually consumes a genealogist's time.

**Duration:** 7 weeks · **Release:** `v0.5`

- [ ] **ONNX Runtime integration** — per-platform execution providers (DirectML, CoreML, NNAPI),
      WebGPU on the web; model fetching as a separate, resumable, verified download
- [ ] **HTR pipeline** — deskew and dewarp, layout analysis, line segmentation, recognition,
      lexicon rescoring against the tree's own names and places, form-aware extraction for
      metric-book templates
- [ ] **Transcription workspace** — every recognised field linked to its image region, keyboard-
      first correction, corrections feeding the fine-tuning set
- [ ] **Record linkage** — Daitch–Mokotoff blocking, Beider–Morse plus Jaro–Winkler comparison,
      interval overlap on dates, place-hierarchy distance, **relationship-overlap scoring**,
      Fellegi–Sunter weights with EM estimation, and **explained scores** in the UI
- [ ] **Duplicate detection** — proposals only, never automatic merges
- [ ] **Faces** — on-device detection, embedding, clustering, identity proposals from already
      tagged people
- [ ] **Photo pipeline** — scan enhancement, dust and scratch removal, colourisation and
      restoration as labelled derivatives; originals immutable
- [ ] **Optional API key** — difficult-hand assistance, document interpretation, proof-argument
      drafting; output always entering as low-confidence attributed assertions

### Exit criteria

A page from a real Russian metric book transcribes with the fields correctly identified and the
errors correctable faster than typing from scratch. Duplicate detection finds the planted
duplicates in a test corpus without false positives above threshold.

---

## Phase 6 — Огранка · *The Cut*

**Goal:** 1.0. Fast, accessible, localised, distributed, documented.

**Duration:** 6 weeks · **Release:** **`v1.0`**

- [ ] **Performance** — profile every hot path; frame-time budget enforced per commit; cold start
      under 1.5 s; memory ceiling on 200 000-person trees
- [ ] **Accessibility audit** — full WCAG 2.2 AA conformance report, AAA where reached; screen
      reader passes on NVDA, VoiceOver and TalkBack; complete keyboard traversal
- [ ] **Localisation QA** — native review of ru, en, de; pseudo-localisation in CI; every
      genealogical formatting rule verified per locale
- [ ] **Distribution** — Microsoft Store, Apple App Store, Google Play, Flathub, plus direct
      downloads; auto-update with signature verification; the PWA served cross-origin isolated
- [ ] **Documentation** — a user guide written for the persona, not for us; an administrator
      guide for self-hosting; developer documentation; a complete API reference
- [ ] **Security review** — dependency audit, penetration test of the server, a threat model
      written down
- [ ] **Migration guides** — from Gramps, MyHeritage, MacFamilyTree, Ancestris, webtrees, each
      tested with a real file
- [ ] **The project site** — what it is, who it is for, how to start, how to self-host

### Exit criteria

The persona uses it for a month without help. A working genealogist uses it for a real client
project. A family of ten synchronises without an administrator.

---

## After 1.0 — candidates, not commitments

| Candidate | Note |
|---|---|
| DNA match import and genetic relationship modelling | Import only; no sequencing |
| Archive-catalogue integrations | Where public APIs exist |
| Community historical-strata datasets | Per-region, versioned, contributed |
| Collaborative transcription | Several people on one document set |
| Tree-structure analysis | Endogamy detection, pedigree collapse reporting, demographic statistics |
| Additional locales | Architecture already assumes them |
| Plugin API | Only once the core API has proven stable across two minor versions |

## Risk register

| Risk | Impact | Mitigation |
|---|---|---|
| Loro fails the Phase 0 spike | High | Automerge 3.5 fallback, abstraction boundary designed for the swap, 2-day re-spike budgeted |
| Own layout engine takes longer than 2.5 weeks | Medium | Ship Phase 1 with a simpler coordinate assignment; crossing reduction is an independent improvement |
| WASM bundle exceeds the first-paint budget | Medium | Eager kernel plus lazily loaded modules; native builds are unaffected |
| Cyrillic HTR accuracy is too low to be useful | Medium | Non-Latin tooling lags Latin by years and we knew it going in. Ship the transcription workspace regardless — structured, image-linked manual transcription is valuable on its own; recognition is an accelerator, not the feature |
| Scope grows without bound | **High** | Phase exit criteria are contracts. Anything not in a phase goes to the post-1.0 list, and the list is allowed to be long |
| Solo-developer bus factor | High | Documentation as a first-class deliverable; ADRs for every decision; no undocumented tribal knowledge |
| App-store rejection on mobile | Low | Tauri mobile is shipping in production apps; the Phase 0 capability matrix surfaces problems in week 3, not month 11 |
