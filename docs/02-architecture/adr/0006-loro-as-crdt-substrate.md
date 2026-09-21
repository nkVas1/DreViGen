# ADR 0006 — Loro as the CRDT substrate

**Status:** Accepted · 2026-09-21 · **conditional on on-device validation (task P0-S1b)**

## Context

[ADR 0003](./0003-contribution-review-over-auto-merge.md) settled that DreViGen converges
automatically within one identity and reviews between identities. It did not settle which library
does the converging. The sync research note named **Loro 1.16** with **Automerge** as the
validated fallback, explicitly conditional on measurement, and the roadmap made that measurement
spike **S1** with a stated budget:

> A 50 000-person tree with a ~100 000-operation history must fork, diff and merge in under 2 s
> using under 300 MB, on a mid-range Android device.

## The measurement

`spikes/s1-crdt-scale`, run 2026-09-21. Loro 1.16.2 against Automerge 0.12.0, on synthetic trees
from `drevigen-testkit` with six scalar fields per person — close to what the canvas actually
reads at z2 and z3. The 50 000-person case produces **350 000 operations**, three and a half
times the budget's history. Memory is measured with a counting global allocator, separating
allocation *churn* (peak) from what the document actually *retains*.

Desktop: the development machine, release build, single-threaded.

| 50 000 people · 350 000 ops | Loro 1.16.2 | Automerge 0.12.0 | Ratio |
|---|---|---|---|
| Build the document | **181 ms** | 2 846 ms | **15.7× faster** |
| Save / snapshot | 165 ms | 114 ms | 0.7× |
| Load a fresh replica | **2.8 ms** | 146 ms | **52× faster** |
| Fork two working copies | 9.5 ms | 4.7 ms | 0.5× |
| Edit 1 000 people on each branch | 87 ms | 76 ms | 0.9× |
| **Merge both branches** | **2.6 ms** | 66.9 ms | **26× faster** |
| **Retained memory** | **95.2 MiB** | 114.4 MiB | **17 % leaner** |
| Snapshot size, full history | 6.6 MiB | **0.8 MiB** | **8× larger** |
| Snapshot size, shallow | 4.5 MiB | — | |

Both libraries clear the merge and memory budgets on the desktop by a wide margin. The
differences that matter are elsewhere.

## Decision

**Loro 1.16 is the CRDT substrate.** Automerge is no longer the fallback of record; it remains a
credible alternative should Loro's development stall.

The decision rests on three findings rather than on the merge number, which both libraries pass:

1. **Build and load time decide the import experience, and the gap is an order of magnitude.**
   Importing a 50 000-person GEDCOM costs 181 ms with Loro and 2.8 s with Automerge. Scaled by
   the 3–5× a mid-range phone typically costs on single-threaded work, that is roughly 0.7 s
   against 11 s. The second number is not a slow import; it is a broken one, and it lands on the
   first thing a new user ever does.
2. **Retained memory favours Loro**, which contradicts the provisional expectation recorded in
   the research note. The earlier impression came from peak allocation churn (175 MiB for Loro
   against 114 MiB for Automerge); separating churn from retention reverses the result — Loro
   holds 95 MiB where Automerge holds 114 MiB.
3. **Loro's primitives match the product's shape**, which was the original argument and still
   stands: a personal working copy is a fork, starting a sync is a diff between frontiers, and
   browsing history is time travel. Nothing had to be built around the library to run this spike.

## Consequences

**Gained.** Headroom on every time-critical path. A merge that is effectively instantaneous, so
the Contribution flow is never waiting on the CRDT — its latency will be semantic diff and human
review, which is where it belongs. Fork, frontier and time-travel primitives that map onto the
product without an adapter layer.

**Given up — and this is a real cost.** Encoded size. Automerge stores the same 50 000-person
document with full history in **0.8 MiB** against Loro's **6.6 MiB**: eight times smaller.
Trimming history with a shallow snapshot only brings Loro to 4.5 MiB, so the gap is in the
encoding, not the history.

**Mitigations.**

- The `.dvg` container stores a **shallow snapshot plus the operation log as separate members**,
  so the log compresses independently and can be fetched lazily. The state a reader needs to open
  a tree is not the state an auditor needs to inspect its history.
- **Sync transfers updates, not snapshots.** The full snapshot is a one-time cost on first pull;
  everything after is a delta against a version vector.
- 6.6 MiB for a 50 000-person archive is acceptable as a one-time download even on a metered
  mobile connection, and a 50 000-person tree is already the extreme end of our target range.
- Snapshot size is tracked as a budgeted metric in CI, so regression is visible rather than
  discovered.

**A supply-chain finding, recorded rather than buried.** `cargo deny` reports four
`unmaintained` advisories in Loro's dependency graph — none of them a vulnerability:

| Crate | Advisory | How it arrives |
|---|---|---|
| `im` 15.1.0 | RUSTSEC-2026-0248 | **Mandatory runtime dependency** of `loro-internal` |
| `sized-chunks` 0.6.5 | RUSTSEC-2026-0251 | via `im` |
| `bitmaps` 2.1.0 | RUSTSEC-2026-0247 | via `im` |
| `atomic-polyfill` 1.0.3 | RUSTSEC-2023-0089 | target-gated; absent from our host builds |

All three of the first group are archived crates by the same author. There is nothing to upgrade
to from our side — replacing `im` is upstream's decision. This does not reverse the choice:
unmaintained is not vulnerable, and `im` is a long-established immutable-collections crate. It
does mean Loro carries a small amount of bit-rot risk that Automerge does not, and it is a thing
to re-examine at every Loro upgrade.

The four are acknowledged individually in `deny.toml` with dated reasons rather than by
downgrading the `unmaintained` check, so **a new unmaintained crate entering the graph still
fails the build**.

**Not yet answered.** These are desktop numbers. The stated criterion is a mid-range Android
device, and nothing here proves the ratio holds under a phone's memory pressure, thermal limits
and slower storage. **Task P0-S1b runs the same spike on device before Phase 0 closes.** If it
fails there, this ADR is superseded rather than amended.

## Alternatives considered

- **Automerge 0.12.** Better encoding, and an order of magnitude slower on the two operations
  that a user waits for. Retained memory is also worse. Stays as a credible alternative.
- **Yjs.** Not benchmarked. Its centre of gravity is collaborative text editing and its history
  model is not built for a review workflow; adopting it would mean fighting the library's design.
  See the sync research note.
- **Our own operation log with no CRDT library.** Tempting, because our operations are coarse and
  our merges are reviewed. Rejected for now: correct causal delivery, compaction and time travel
  are a research project of their own, and Loro provides them at a cost the measurement shows we
  can afford. Revisit only if a concrete Loro limitation forces it.

## Reproducing

```sh
cargo run -p s1-crdt-scale --release
```

The spike prints every phase with time, retained memory, peak churn, encoded size and operation
count, then checks itself against the budget. Trees are seeded, so the numbers above are
reproducible on the same hardware.
