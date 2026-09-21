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
  PHASE 1  Ядро                 █████████████                  11.5 weeks    model, GEDCOM, canvas, Hall, media
  PHASE 2  Мастерская                       ████████              8 weeks    sources, evidence, research
  PHASE 3  Родня                                    ████████      8 weeks    server, accounts, contributions
  PHASE 4  Выражение                                    ██████    6 weeks    charts, print, export, publish
  PHASE 5  Автоматика                                     ██████  7 weeks    HTR, faces, record linkage
  PHASE 6  Огранка                                         █████  6 weeks    performance, a11y, l10n, release
                                                                 ────────
                                                                 49.5 weeks
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
| S1 | ~~**Loro at scale.**~~ **Done 2026-09-21.** Desktop: 50 000 people / 350 000 ops merge in **2.6 ms** retaining **95 MiB**; Automerge is 15.7× slower to build and 26× slower to merge. → [ADR 0006](../02-architecture/adr/0006-loro-as-crdt-substrate.md) | 4 days | **Passed.** Cost: Loro snapshots are 8× larger than Automerge's — mitigated in the ADR |
| S1b | **The same spike on a mid-range Android device.** S1 produced desktop numbers only; the stated criterion is a phone. | 1 day | Merge < 2 s, retained < 300 MB on device. Fail → ADR 0006 is superseded, not amended |
| S2 | ~~**WASM core size.**~~ **Done 2026-09-21.** Chosen dependencies compile to **606 KiB gzipped** against a 2 500 KiB budget — Loro is 589 KiB of it, the testkit 9 KiB. → [ADR 0007](../02-architecture/adr/0007-single-eager-wasm-module.md) | 2 days | **Passed**, 1 894 KiB of headroom. No lazy split. Re-measure as each core crate lands; Beider–Morse rule tables are the one to watch |
| S3 | ~~**MSDF atlas coverage.**~~ **Done 2026-09-21.** ASCII + Russian + German + typographic punctuation = **1.3 MB across six instances at 512²**; adding Greek and Latin Extended costs 1024² atlases and 18 MB of VRAM. → [ADR 0009](../02-architecture/adr/0009-msdf-atlas-tiers.md) | 3 days | **Passed** at a third of budget. Greek loads on demand. Found that msdf-atlas-gen 1.4 silently ignores variable-axis settings |
| S4 | ~~**OPFS concurrency.**~~ **Done 2026-09-21.** Integrity held in all five scenarios; throughput did not. `opfs-sahpool` is **7.1× faster** than `opfs` and needs no cross-origin isolation. → [ADR 0008](../02-architecture/adr/0008-opfs-sahpool-single-connection.md) | 2 days | **Passed.** Refutes two documented assumptions: COOP/COEP is dropped, and the model tightens from single-writer to single-*connection* |
| S5 | ~~**Tauri mobile capability.**~~ **Done 2026-09-21.** Everything is covered by official plugins except **camera, photo library and share sheet**, where no maintained plugin exists. → [mobile-capabilities.md](../02-architecture/mobile-capabilities.md) | 3 days | **Passed.** 12 days of previously unplanned mobile work surfaced, scheduled into Phases 1 and 4 |

### Deliverables

- [ ] pnpm + Cargo workspace, the full directory structure from the architecture overview
- [ ] Toolchain pinned: Rust 1.96.1, Node 24.11.1, pnpm 11.10.0, exact versions throughout
- [x] CI (Rust half): `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, rustdoc
      with `-D warnings`, `cargo deny` for licences, bans, sources and advisories — all blocking
- [ ] CI (web half): `tsc --noEmit`, `eslint`, `vitest`, `pnpm audit`, WASM size budget
- [ ] Release pipeline producing signed artefacts for all six targets, even if the app is empty
- [ ] `tokens` package: the OKLCH → sRGB + APCA pipeline, with contrast failures breaking the build
- [ ] Fonts vendored and subset; the type audit from the art direction resolved
- [ ] The app opens on Windows, Android and in a browser, shows one screen, and is installable
- [ ] P0-S5a — Rust `std::fs` verified inside the app sandbox on both mobile platforms
- [ ] P0-S5b — the Rust WebSocket client verified from both mobile platforms

### Exit criteria

Five ADRs written. The repository builds green on every target. A person can install the app on
a phone and see a correctly typed, correctly coloured screen.

---

## Phase 1 — Ядро · *Core*

**Goal:** a genuinely usable single-user genealogy application. Someone can build a real family
tree in it, import their existing GEDCOM, and show it to relatives.

**Duration:** 11.5 weeks · **Release:** `v0.1`

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
- [ ] Web build on `opfs-sahpool`, one connection in one worker, reads included
- [ ] Web Locks arbitrate tab ownership; a second tab gets a plain-language screen, not a VFS error

**1.5 · Layout engine** (`drevigen-layout`, 2.5 weeks)
- [ ] Generation assignment by longest path
- [ ] Ordering with couples as atomic units; barycentre crossing reduction
- [ ] Coordinate assignment with centred sibling groups
- [ ] **Anchored incremental layout** — the stability property
- [ ] Benchmarks at 1 k / 10 k / 50 k / 200 k; quality compared against Graphviz and ELK

**1.6 · Canvas** (`packages/canvas`, 2.5 weeks)
- [ ] WebGL 2 renderer with instanced symbols; WebGPU behind a capability check
- [ ] MSDF text pipeline: tier A pre-baked at 512², on-demand loading per Unicode block,
      metrics as packed binary rather than the tool's JSON
- [ ] Incremental R-tree for culling and hit-testing
- [ ] All five semantic-zoom bands with cross-fade and no reflow
- [ ] Spring-driven pan and zoom, interruptible, velocity-carrying

**1.8 · Mobile media** (`crates/drevigen-plugin-media`, 1.5 weeks)
- [ ] Camera capture and photo-library picking on iOS and Android, from one Rust surface
- [ ] **EXIF preserved end to end** — capture date and camera model are genealogical evidence
- [ ] Desktop falls back to the `dialog` file picker
- [ ] `tauri-plugin-android-fs` integrated for scoped storage

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
- [ ] `drevigen-plugin-share` — the native share sheet on iOS and Android, since no maintained
      plugin provides one
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
| ~~Loro fails the Phase 0 spike~~ | — | **Retired 2026-09-21.** S1 passed with wide margin; see [ADR 0006](../02-architecture/adr/0006-loro-as-crdt-substrate.md). Residual risk is the on-device run, tracked as S1b |
| Loro snapshots are 8× larger than Automerge's | Medium | Shallow snapshot plus a separate op log in the `.dvg` container; sync transfers deltas not snapshots; snapshot size budgeted in CI |
| Own layout engine takes longer than 2.5 weeks | Medium | Ship Phase 1 with a simpler coordinate assignment; crossing reduction is an independent improvement |
| ~~WASM bundle exceeds the first-paint budget~~ | Low | **Downgraded 2026-09-21.** S2 measured 606 KiB of 2 500 KiB. Residual risk is `drevigen-match`, whose Beider–Morse tables are data rather than code; the size budget becomes a failing CI gate as the core lands |
| Cyrillic HTR accuracy is too low to be useful | Medium | Non-Latin tooling lags Latin by years and we knew it going in. Ship the transcription workspace regardless — structured, image-linked manual transcription is valuable on its own; recognition is an accelerator, not the feature |
| Scope grows without bound | **High** | Phase exit criteria are contracts. Anything not in a phase goes to the post-1.0 list, and the list is allowed to be long |
| Solo-developer bus factor | High | Documentation as a first-class deliverable; ADRs for every decision; no undocumented tribal knowledge |
| App-store rejection on mobile | Low | Tauri mobile is shipping in production apps; the Phase 0 capability matrix surfaced its problems in week 3, not month 11 — see [mobile-capabilities.md](../02-architecture/mobile-capabilities.md) |
| Photo import is the Хранитель persona's whole reason to open the mobile app, and no maintained plugin provides it | Medium | We write `drevigen-plugin-media` ourselves rather than depend on a 0.1.x crate with no commits in fifteen months. 7 days, scheduled in Phase 1 |
