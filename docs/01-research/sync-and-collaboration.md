# Synchronisation, Merging & Attribution

*Access date: 2026-09-21.*

## 1. The requirement, stated precisely

Each participant keeps a **personal working copy** of the family tree and accumulates changes in
it locally, offline, for as long as they like. At a moment of their choosing they start a sync.
The application compares their copy against the **canonical cloud tree**, and:

- where there is no disagreement, changes flow up cleanly;
- where there is disagreement, the user is shown the conflict and offered resolutions;
- the canonical tree is the **verified build** — the version the family treats as true.

Every change that lands must be attributable: who, what, when, why, and against which source.
The history must be browsable like a wiki page history or a git log.

This is a **pull-request model**, and it is the right one. It is also *not* what a CRDT gives
you, and the difference matters enough to spell out.

## 2. Why automatic merge is insufficient here

CRDTs guarantee **convergence** — that two replicas which have seen the same operations agree.
They say nothing about **correctness**. In genealogy the interesting conflicts are semantic and
invisible to any syntactic merge:

- Two relatives independently add great-grandmother. Both edits are valid, neither conflicts in
  any CRDT sense, and the result is a **duplicate person** — the single most common data-quality
  failure in collaborative genealogy.
- One contributor sets a birth year from a census; another sets it from a parish register. A
  last-write-wins map silently discards the loser. The correct outcome is *two assertions and a
  conclusion* (see [data-standards.md](./data-standards.md) §4), which no generic merge can
  synthesise.
- One contributor attaches a child to the wrong family. The merge is clean; the genealogy is now
  wrong, and wrong in a way that propagates.

**[judgement]** Therefore: CRDT machinery for **convergence and causality**, a semantic diff and
human review for **correctness**. Automatic where it is safe, reviewed where it is not.

## 3. Where automatic merge *is* right

One user, several devices — laptop, phone, tablet. Here review would be absurd friction. Edits
from a single identity across that user's own devices merge automatically and silently. The
**personal working copy is itself a distributed, auto-converging document**; only its promotion
to canonical is reviewed.

This split gives a clean rule:

> **Within one identity: converge. Between identities: review.**

## 4. CRDT substrate — survey and choice

| Library | Version | Assessment |
|---|---|---|
| **Yjs** | 13.6.32 | The production default with by far the largest ecosystem, but its centre of gravity is collaborative text editing (ProseMirror, TipTap, CodeMirror, Monaco bindings). Its history model is not designed for review workflows. |
| **Automerge** | 3.5.0 | Document-level versioning with a git-like change history; 3.0 (May 2025) cut memory roughly 10× via a Rust core, making large documents practical. Strong fit conceptually. |
| **Loro** | 1.16.1 | Rust core with JS/WASM, Swift and Python bindings. Explicitly built around **git-like history**: version vectors and frontiers, **time travel to any point**, **forking a document into a branch** and merging it back, plus a **MovableTree** CRDT for hierarchical data and Fugue-based text. Fastest of the three in published benchmarks; youngest ecosystem. |

**Decision: Loro, with Automerge 3.5 as the validated fallback.** **[judgement]**

The reasoning is that Loro's primitives are a near-exact match for the product's shape. A
personal working copy *is* a fork. Starting a sync *is* computing the diff between two
frontiers. Browsing history *is* time travel. Building the pull-request model on Automerge is
possible but requires more scaffolding; on Yjs it would mean fighting the library's design.

**This choice is conditional on a Phase 0 spike** (see the roadmap): a 50 000-person tree with
a 100 000-operation history must import, fork, diff and merge within defined time and memory
budgets on a mid-range phone. If Loro fails that bar, Automerge 3.5 is the fallback and the
abstraction boundary is designed so the swap costs days, not months.

## 5. Attribution that survives merging

CRDT operations carry a peer identifier. That is the hook: **peer ID binds to an account**, and
every operation therefore carries a verifiable author. On top of the operation log we maintain a
derived, queryable **attribution index**:

```
(object_id, field_path) → [ (author, timestamp, op_id, contribution_id, citation?) … ]
```

This is the same problem Wikipedia solves for text — the WikiWho line of work computes token-level
provenance across revision histories — but far easier for us, because our units are structured
fields rather than free text. **[established]**

It yields, directly:

- **Blame view.** Point at any fact and see who asserted it, when, and from which source.
- **Contributor pages.** What has this person added; which of their contributions are cited.
- **Trust signals.** Not a reputation score — an honest display of sourcing rate and review
  history. Research on uncertainty in collaborative platforms supports surfacing source
  reliability rather than hiding it behind an aggregate.
- **Revert.** Any contribution, or any single change inside one, can be reverted as a new
  attributed change. Nothing is ever erased from history.

## 6. The Contribution (pull-request) flow

```
  Local working copy                         Canonical tree (cloud)
  ─────────────────                          ──────────────────────
  edit, edit, edit          ──┐
  (offline, days or weeks)    │
                              │
  «Отправить изменения»  ─────┼──►  1. fetch canonical frontier
                              │     2. semantic diff  (local ⊕ canonical)
                              │     3. classify each change
                              │            ├─ clean        → auto-stage
                              │            ├─ conflicting  → resolution card
                              │            └─ suspicious   → validator warning
                              │     4. author reviews the summary, writes a note
                              │     5. Contribution opens
                              │
                              └──►  6. reviewers comment / request changes / approve
                                    7. merge → new canonical version
                                    8. every other copy sees it on next sync
```

### Change classification

| Class | Examples | Handling |
|---|---|---|
| **Clean** | New person nobody else touched; a note added; a photo attached | Auto-staged, still listed |
| **Conflicting** | Same field set to different values; same person deleted here and edited there | Resolution card: keep mine / keep theirs / **keep both as competing assertions** / park as open question |
| **Suspicious** | Probable duplicate person; child born before parent; implausible lifespan; unsourced change to a sourced fact | Validator warning, blocking only if the tree's policy says so |

**"Keep both as competing assertions" is the option no existing tool offers**, and it is usually
the honest answer. It converts a merge conflict into properly modelled data.

### Roles

| Role | Can |
|---|---|
| **Хранитель** (owner) | Everything, including policy, membership and erasure |
| **Редактор** (maintainer) | Review and merge contributions; edit canonical directly |
| **Участник** (contributor) | Open contributions; comment; edit own working copy freely |
| **Гость** (viewer) | Read at the visibility level granted; comment; suggest a correction without a full contribution |

A viewer's "suggest a correction" path matters: a distant relative who spots a wrong date should
be able to say so in two clicks, without learning the contribution model. It arrives as a
lightweight, single-fact contribution.

## 7. Server architecture

Deployment target is the owner's existing **Selectel VPS**, self-hosted, with a documented
one-command install.

```
                    ┌──────────────────────────────────────────┐
   clients  ──TLS──►│  Caddy  (automatic certificates)         │
                    └───────────────┬──────────────────────────┘
                                    │
                    ┌───────────────▼──────────────────────────┐
                    │  drevigen-server   (Rust, Axum, Tokio)   │
                    │   • accounts, invitations, roles          │
                    │   • contribution review API               │
                    │   • operation-log sync endpoints          │
                    │   • media upload / derivative pipeline    │
                    │   • WebSocket presence & notifications    │
                    └───┬──────────────┬───────────────┬────────┘
                        │              │               │
                ┌───────▼──────┐ ┌─────▼──────┐ ┌──────▼───────┐
                │ PostgreSQL   │ │ Object     │ │ Valkey       │
                │ 18           │ │ store      │ │ (queues,     │
                │ metadata,    │ │ (fs or S3- │ │  sessions,   │
                │ op log,      │ │ compatible)│ │  rate limit) │
                │ attribution  │ │ media blobs│ │              │
                └──────────────┘ └────────────┘ └──────────────┘
```

**Stack rationale.** The 2026 consensus for new Rust backends is **Axum + Tokio + SQLx +
PostgreSQL**; Axum is Tokio-native, type-safe in its extractors and maintained for the long term,
and SQLx validates queries at compile time. A Rust binary containerises to a small image with no
interpreter or GC, which matters on a modest VPS. CPU-bound work (image derivatives, HTR,
matching) must go through `spawn_blocking` or a separate worker rather than blocking the async
runtime. **[established]**

**Why not Cloudflare Durable Objects**, despite a genuinely capable free tier (Durable Objects
reached GA on 2025-07-15 and are available on the Workers free plan with SQLite storage;
incoming WebSocket messages bill at 20:1): the owner already has a VPS, self-hosting keeps the
sovereignty guarantee absolute, and a single Rust binary is easier to reason about than a
distributed-object topology. **We keep a Workers deployment target as a documented alternative**
for people who have no server — it costs us an adapter, not an architecture. **[current]**

**Media.** Content-addressed by BLAKE3 hash, deduplicated, with derivatives (thumbnails, display
sizes, AVIF/WebP) generated on upload. The interface is an object-store trait; the default
implementation is the local filesystem, with an S3-compatible implementation for anyone who
wants MinIO or a bucket.

## 8. Local storage

| Concern | Choice |
|---|---|
| Native (Tauri) | SQLite via the Rust core, WAL mode, FTS5 for search |
| Web | SQLite compiled to WASM over **OPFS**, in a worker |
| Operation log | Append-only, content-addressed, stored alongside the snapshot |
| Project file | `.dvg` — a zip container: snapshot + operation log + media + manifest, and a valid `.gdz` superset so GEDCOM 7 tools can read the genealogical core |

On the web path, SQLite WASM over OPFS is now a serious runtime — multi-gigabyte client-side
databases at near-native speed. Two operational facts govern the implementation: maximum
performance needs `SharedArrayBuffer`, which requires **cross-origin isolation** (COOP/COEP
headers); and testing in March 2026 showed 8–10 concurrent workers are sustainable provided
locking is minimal and `SQLITE_BUSY` is handled explicitly. **[current]**

**Obligation.** Serve the web build cross-origin isolated; keep exactly one writer worker;
handle `SQLITE_BUSY` with backoff rather than assuming it cannot happen.

## 9. Summary of obligations

| Requirement | Decision |
|---|---|
| Convergence substrate | Loro 1.16.1 (fork/merge/time-travel), Automerge 3.5 fallback, validated by a Phase 0 spike |
| Within one identity | Automatic silent merge across that user's devices |
| Between identities | Contribution review flow with semantic diff and change classification |
| Conflict resolution | Mine / theirs / **both as competing assertions** / park as open question |
| Attribution | Peer ID bound to account; derived attribution index; blame, revert, contributor pages |
| Server | Rust + Axum + SQLx + PostgreSQL 18 + Valkey, Docker Compose, Caddy, on the owner's VPS |
| Alternative deployment | Documented Cloudflare Workers adapter for users without a server |
| Local storage | SQLite (native) / SQLite WASM + OPFS (web), FTS5, append-only op log |
| Project file | `.dvg` zip container, superset of GEDCOM 7 `.gdz` |
