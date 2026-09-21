# ADR 0008 — One SQLite connection on `opfs-sahpool`, and no cross-origin isolation

**Status:** Accepted · 2026-09-21 · supersedes the cross-origin-isolation requirement recorded
in [overview.md](../overview.md) §3 and
[sync-and-collaboration.md](../../01-research/sync-and-collaboration.md) §8

## Context

The web build stores its database in the browser through SQLite compiled to WebAssembly. The
research note recorded two claims from secondary sources, both of which turn out to be wrong in
the direction that matters:

> Maximum performance needs `SharedArrayBuffer`, which requires **cross-origin isolation**
> (COOP/COEP); and testing in March 2026 showed 8–10 concurrent workers are sustainable provided
> locking is minimal and `SQLITE_BUSY` is handled explicitly.

Spike **S4** was written to confirm the single-writer model and the `SQLITE_BUSY` handling. It
confirmed the first and refuted the framing of the second.

## The measurement

`spikes/s4-opfs-concurrency`, run 2026-09-21 in Chromium, cross-origin isolated, SQLite 3.53.4
via `@sqlite.org/sqlite-wasm`. Each scenario writes **10 000 rows** in 250-row transactions and
then verifies the result by row count, distinct count and a gap scan — the last catching a
committed transaction that silently lost part of its batch, which a row count alone would miss.

| # | VFS | Connections | Throughput | Integrity |
|---|---|---|---|---|
| A | `opfs` | 1 | 2 358 rows/s | **intact** |
| B | `opfs` | 1 writer + 6 readers | **356 rows/s** | **intact** |
| C | `opfs` | 4 writers + 6 readers | 342 rows/s | **intact** |
| D | `opfs-sahpool` | 1 | **16 849 rows/s** | **intact** |
| E | `opfs-sahpool` | a second connection | **refused outright** | — |

Four things fall out of this, and three of them were not what we expected.

**Integrity is not the problem.** Every scenario wrote all 10 000 rows with no gaps and no
duplicates, including four writers competing for one file. SQLite's locking does its job. The
pass criterion — 10 000 writes, no loss, no deadlock — is met by every configuration tested.

**Concurrency is the problem, and it is not write contention.** Going from one connection to
seven costs **6.6×** (2 358 → 356 rows/s). Going from there to ten connections with four of them
writing costs almost nothing more (356 → 342). The penalty is for *having more than one
connection at all*, not for writing from several. The cause is visible in the console: the
`opfs` VFS emits `GetSyncHandleError … Access Handles cannot be created if there is another open
Access Handle` and retries, because the browser permits exactly one `SyncAccessHandle` per file
and the VFS acquires and releases it around every lock.

**`opfs-sahpool` is 7.1× faster than single-connection `opfs`** and 47× faster than the
multi-connection case — and it needs no cross-origin isolation, because it does not use
`SharedArrayBuffer`. Its constraint is that a pool belongs to one connection, which scenario E
confirms: a second connection to the same pool is refused immediately with a clear error rather
than degrading or corrupting.

**WAL is unavailable.** Both VFS variants report `journal_mode=delete` after an explicit
`PRAGMA journal_mode = WAL`. WAL needs shared memory, which OPFS does not provide. The web build
therefore has different durability characteristics from the native build, where WAL is used.

## Decision

1. **The web build uses `opfs-sahpool`**, not `opfs`.
2. **Exactly one SQLite connection exists per origin**, owned by one dedicated worker. Reads go
   through it like writes. This is stronger than the single-*writer* model we had written down.
3. **The web build is not served cross-origin isolated.** COOP and COEP are dropped.
4. **Multi-tab ownership is arbitrated explicitly** with the Web Locks API.

## Consequences

**Gained — and this is the significant one: COOP/COEP goes away.** Cross-origin isolation is an
invasive constraint. It blocks embedding any cross-origin resource that does not opt in with
CORP, complicates hosting, and interacts badly with embedded maps, fonts and archive imagery —
all of which a genealogy application wants. Discovering that the *faster* option is also the
*unconstrained* one is a straightforward win, and we would have paid for the constraint
needlessly for the life of the project.

**Gained.** 16 849 rows/s is comfortable headroom: a 50 000-person GEDCOM import is a few
seconds of database work rather than a progress bar with a cancel button.

**Given up.** Multiple connections, which we did not want. And WAL, which we do not get a choice
about.

**The cost is multi-tab handling, and it lands on the persona we care about most.** Opening the
application in a second tab will fail to acquire the pool. The failure is loud, which is right —
but *"Failed to execute createSyncAccessHandle on FileSystemFileHandle"* reaching our primary
persona would be a total failure of the product's central promise. So:

- The owning tab holds a **Web Lock** on the tree's identifier for as long as it has the
  database open.
- A second tab detects the held lock **before touching storage** and presents a plain-language
  screen: *«Древо уже открыто в другой вкладке»*, with one button that focuses the owning tab and
  one that takes over. No error code, no jargon, no dead end — the rule from
  [elder-ux-and-accessibility.md](../../01-research/elder-ux-and-accessibility.md).
- Taking over releases the lock in the first tab, which closes its connection cleanly and
  switches to the same message.

**Native is unaffected.** The Tauri builds use SQLite directly with WAL, multiple connections
and no VFS gymnastics. The web path is the constrained one, and `drevigen-store` hides the
difference behind one interface — which is exactly the split
[ADR 0001](./0001-rust-core-on-every-platform.md) exists to maintain.

**Corrections this forces.** Two documents assert that the web build requires cross-origin
isolation. Both are wrong and are amended to point here.

## Alternatives considered

- **`opfs` with one connection.** Works, keeps multi-connection open as a future option, and
  costs 7× throughput plus the COOP/COEP constraint for an option we have decided not to use.
- **`opfs` with several connections**, as the secondary source suggested was sustainable. It
  *is* sustainable in the sense that nothing breaks — and it costs 85 % of throughput. "Works"
  and "is a good idea" are different findings, and this is why the spike measured rather than
  read.
- **IndexedDB rather than SQLite.** Loses SQL, FTS5 and the shared schema with the native build,
  which is most of what `drevigen-store` is for.
- **Keeping everything in memory and persisting snapshots.** Viable while a tree is small, and
  it discards the incremental durability that makes an archive trustworthy.

## Reproducing

```sh
pnpm --filter s4-opfs-concurrency start
# then open http://127.0.0.1:8732/
```

The page runs all five scenarios and prints a verdict. It counts VFS-level lock failures
separately from `SQLITE_BUSY`, because the `opfs` VFS reports lock trouble through
`console.error` rather than by failing a call — a number that does not appear unless you watch
for it.
