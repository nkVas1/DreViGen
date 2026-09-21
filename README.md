<div align="center">

# DreViGen

**A family archive you can navigate like a map, keep like an heirloom, and research like a scholar.**

*Local-first genealogy for Windows, macOS, Linux, iOS, Android and the web — from one codebase.*

[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-4E555F.svg?style=flat-square)](./LICENSE)
[![Status: Phase 0](https://img.shields.io/badge/status-phase%200%20·%20foundation-B4552C?style=flat-square)](./docs/04-planning/roadmap.md)
[![Docs](https://img.shields.io/badge/docs-read-6B8163?style=flat-square)](./docs/)

[Vision](./docs/00-vision.md) · [Research](./docs/01-research/) · [Architecture](./docs/02-architecture/overview.md) · [Design](./docs/03-design/art-direction.md) · [Roadmap](./docs/04-planning/roadmap.md) · [Русский](./README.ru.md)

</div>

---

> **Project status — early.** DreViGen is in Phase 0: the foundation and the risky spikes.
> The documentation below describes what is being built and why. There is no installable
> release yet; the [roadmap](./docs/04-planning/roadmap.md) says when there will be.

## Why this exists

Genealogy software splits into two camps. **Consumer platforms** are welcoming and beautiful,
and they are subscription funnels where your life's work lives on someone else's server and
exports as a lossy file. **Serious tools** have real scholarly depth and interfaces that assume
you read the manual first.

Nobody has built the tool that serves both, in the same document, without asking either one to
compromise. That is the whole point of DreViGen.

## What makes it different

**Evidence is modelled, not flattened.** Every other tool stores your *conclusion* and staples
sources to it. DreViGen keeps **Assertions** (what each source says) separate from
**Conclusions** (what you decided, and why). When the census says 1873 and the parish register
says 1871, both survive, the disagreement is visible, and your reasoning is recorded. This is
what the [Genealogical Proof Standard](https://www.familysearch.org/en/wiki/Genealogical_Proof_Standard)
actually requires, and it is unimplementable in a flat data model.

**The tree is a territory, not a diagram.** Fifty thousand people get cartographic treatment:
five bands of **semantic zoom** where the scale changes the *symbol* rather than its size, a
minimap, a legend that explains every mark in words, named landmarks, and a generation scale.
Adding one person never moves anyone else.

**Collaboration works like a pull request.** Each participant keeps a personal working copy and
edits offline for as long as they like. When they sync, a **semantic diff** against the family's
canonical tree classifies every change as clean, conflicting or suspicious, and conflicts offer
an option no other tool has: *keep both as competing assertions*. Every fact is attributable —
who asserted it, when, from which source.

**Two front doors, one document.** The **Hall** is for family members who want to see faces and
understand how they are related. The **Workshop** is for researchers who need citations,
contradiction queues, research logs and proof arguments. Neither compromises for the other.

**It is yours.** Local-first, offline by default, lossless GEDCOM 7 in and out, self-hostable in
one command. No mandatory subscription, no mandatory cloud, no paid dependency anywhere in the
stack.

## Design

DreViGen looks like a 19th-century scientific atlas, and that is an engineering decision as much
as an aesthetic one. Research on older users finds a measurable preference for **high contrast,
skeuomorphic cues and labelled icons** — so a plate, a specimen card and a handwritten margin
note are forms our primary user already understands. The 2026 movement away from
AI-generated sameness converges on **tactile materiality and craft**. For once, the
accessibility requirement and the originality requirement point the same way.

Read the full system in [art-direction.md](./docs/03-design/art-direction.md).

## Technology

| Layer | Choice |
|---|---|
| **Core** | Rust — domain model, GEDCOM 7, calendars, layout, matching, sync. Compiled natively **and** to WASM, so correctness cannot differ between platforms. |
| **Shell** | Tauri 2 → Windows, macOS, Linux, iOS, Android · installable PWA from the same front end |
| **UI** | React 19 + React Compiler, **React Aria** (chosen specifically for accessibility), Tailwind 4, Motion 13, TanStack Router/Query |
| **Canvas** | Own renderer over WebGL 2, WebGPU behind a capability check. Instanced symbols, MSDF text, R-tree culling. |
| **Storage** | SQLite with FTS5 (native) · SQLite WASM over OPFS (web) · append-only operation log |
| **Sync** | Loro CRDT for convergence, semantic diff and human review for correctness |
| **Server** | Rust · Axum · SQLx · PostgreSQL 18 · Valkey · Caddy — self-hosted, one command |

Every choice is argued in [`docs/02-architecture/`](./docs/02-architecture/), and the research
behind it in [`docs/01-research/`](./docs/01-research/).

## Building it

There is no release yet — see the status note above — but the application builds and opens
today, on Windows and in a browser, from one front end.

You need [Rust](https://rustup.rs) (the version in `rust-toolchain.toml` is installed
automatically), [Node 24](https://nodejs.org) and [pnpm 11](https://pnpm.io). A native build
additionally needs the
[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your platform.

```sh
pnpm install

pnpm --filter @drevigen/web dev      # the web build, on http://localhost:5173
pnpm --filter @drevigen/shell dev    # the native window, same front end
```

Everything that gates a pull request runs locally in one line each:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo deny check                      # licences, bans, sources, advisories
cargo run -p drevigen-tokens -- check # every colour pair is readable
pnpm --filter @drevigen/web typecheck
```

Two things in the repository are **generated and committed**, so that a change to either is
visible in a pull request rather than at release time:

```sh
cargo run -p drevigen-tokens -- generate                                   # the palette
cargo run -p drevigen-mark --features render -- assets/identity apps/web/public  # the mark
```

## Documentation

| | |
|---|---|
| [**Vision**](./docs/00-vision.md) | The problem, the personas, the principles, the non-goals |
| [**R&D dossier**](./docs/01-research/) | Competitive landscape · data standards · layout and rendering · sync · elder UX · 2026 design language · ML · [bibliography](./docs/01-research/sources.md) |
| [**Architecture**](./docs/02-architecture/overview.md) | System shape, stack rationale, [ADRs](./docs/02-architecture/adr/) |
| [**Design**](./docs/03-design/art-direction.md) | Art direction · [asset brief](./docs/03-design/asset-brief.md) |
| [**Roadmap**](./docs/04-planning/roadmap.md) | Seven phases to 1.0, with exit criteria and a risk register |

## Contributing

Not yet open for code contributions — the foundation is still being laid. Issues, questions and
argument about the design are welcome now. See [CONTRIBUTING.md](./CONTRIBUTING.md).

## Licence

[GNU Affero General Public License v3.0](./LICENSE). DreViGen ships a self-hostable sync server,
and the network-use clause keeps hosted derivatives open. It also matches the licensing of the
established open-source genealogy ecosystem — Gramps, webtrees and Ancestris are all GPL-family.

---

<div align="center">
<sub><b>Дре</b>во · <b>Vi</b>sual · <b>Gen</b>ealogy</sub>
</div>
