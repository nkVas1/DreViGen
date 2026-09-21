// One SQLite connection, living in a worker, doing what the harness tells it.
//
// Both OPFS VFS variants are supported so the spike can compare them:
//
//   opfs          SyncAccessHandle-backed, needs cross-origin isolation, supports several
//                 concurrent connections with SQLite's own locking.
//   opfs-sahpool  A pool of pre-opened access handles. No cross-origin isolation needed and
//                 usually faster, but a pool belongs to one connection — which is exactly the
//                 constraint the single-writer model already imposes on us.
//
// Whether that second option is viable decides whether the whole web build has to be served
// cross-origin isolated, so it is worth measuring rather than assuming.

import sqlite3InitModule from './node_modules/@sqlite.org/sqlite-wasm/dist/index.mjs';

let sqlite3 = null;
let db = null;
let label = 'unnamed';

/** Rows written per transaction. Batching is the difference between usable and unusable. */
const BATCH = 250;

async function open({ vfs, file, busyTimeoutMs, name, poolName }) {
  label = name;
  sqlite3 ??= await sqlite3InitModule();

  if (vfs === 'opfs-sahpool') {
    // The pool name is the resource two tabs would contend for, so the caller controls it:
    // defaulting it per worker would silently give each connection its own pool and turn the
    // second-connection test into a test of nothing.
    const pool = await sqlite3.installOpfsSAHPoolVfs({ name: poolName ?? `s4-${name}` });
    db = new pool.OpfsSAHPoolDb(file);
  } else {
    if (!sqlite3.oo1.OpfsDb) {
      throw new Error(
        'opfs VFS unavailable — the document is probably not cross-origin isolated',
      );
    }
    db = new sqlite3.oo1.OpfsDb(file);
  }

  // Without a busy timeout, a second writer fails instantly instead of waiting its turn.
  db.exec(`PRAGMA busy_timeout = ${busyTimeoutMs}`);
  db.exec('PRAGMA journal_mode = WAL');
  db.exec('PRAGMA synchronous = NORMAL');

  return { version: sqlite3.version.libVersion, journal: readJournalMode() };
}

function readJournalMode() {
  let mode = '?';
  db.exec({ sql: 'PRAGMA journal_mode', callback: (row) => { mode = row[0]; } });
  return mode;
}

function createSchema() {
  db.exec(`
    CREATE TABLE IF NOT EXISTS person (
      id      INTEGER PRIMARY KEY,
      writer  TEXT    NOT NULL,
      surname TEXT    NOT NULL,
      born    INTEGER NOT NULL
    );
  `);
  db.exec('DELETE FROM person');
  return { ok: true };
}

/**
 * Inserts `count` rows with ids in `[from, from + count)`, retrying on SQLITE_BUSY.
 *
 * Returns the observed contention, which is the interesting number: a single-writer run should
 * report zero, and a contended run should report retries rather than losses.
 */
function write({ from, count, writer }) {
  let busy = 0;
  let retries = 0;
  const started = performance.now();

  for (let offset = 0; offset < count; offset += BATCH) {
    const size = Math.min(BATCH, count - offset);
    let attempt = 0;

    for (;;) {
      try {
        db.exec('BEGIN IMMEDIATE');
        const stmt = db.prepare(
          'INSERT OR REPLACE INTO person (id, writer, surname, born) VALUES (?, ?, ?, ?)',
        );
        try {
          for (let i = 0; i < size; i += 1) {
            const id = from + offset + i;
            stmt.bind([id, writer, `Васильев-${id}`, 1800 + (id % 200)]).stepReset();
          }
        } finally {
          stmt.finalize();
        }
        db.exec('COMMIT');
        break;
      } catch (error) {
        try {
          db.exec('ROLLBACK');
        } catch {
          // No transaction was open; nothing to undo.
        }
        if (!isBusy(error)) throw error;
        busy += 1;
        attempt += 1;
        retries += 1;
        if (attempt > 40) {
          throw new Error(`gave up after ${attempt} busy retries: ${error.message}`);
        }
        sleep(Math.min(2 ** attempt, 64) + Math.random() * 16);
      }
    }
  }

  return { writer, count, busy, retries, ms: performance.now() - started };
}

function read({ rounds }) {
  let rows = 0;
  let busy = 0;
  const started = performance.now();
  for (let i = 0; i < rounds; i += 1) {
    try {
      db.exec({
        sql: 'SELECT count(*) FROM person WHERE born > ?',
        bind: [1900],
        callback: (row) => { rows += row[0]; },
      });
    } catch (error) {
      if (!isBusy(error)) throw error;
      busy += 1;
    }
  }
  return { rounds, rows, busy, ms: performance.now() - started };
}

function verify({ expected }) {
  let total = 0;
  let distinct = 0;
  let minId = -1;
  let maxId = -1;
  db.exec({ sql: 'SELECT count(*), count(DISTINCT id), min(id), max(id) FROM person',
    callback: (row) => { [total, distinct, minId, maxId] = row; } });

  // A gap check catches the failure mode that a row count alone would miss: a committed
  // transaction that silently lost part of its batch.
  let gaps = 0;
  db.exec({
    sql: `SELECT count(*) FROM (
            SELECT id, lag(id) OVER (ORDER BY id) AS prev FROM person
          ) WHERE prev IS NOT NULL AND id <> prev + 1`,
    callback: (row) => { gaps = row[0]; },
  });

  return { total, distinct, minId, maxId, gaps, expected, intact: total === expected && gaps === 0 };
}

function close() {
  db?.close();
  db = null;
  return { ok: true };
}

function isBusy(error) {
  const code = error?.resultCode ?? error?.result ?? 0;
  // 5 = SQLITE_BUSY, 6 = SQLITE_LOCKED, 261/262 = the extended forms.
  return (
    code === 5 || code === 6 || code === 261 || code === 262 ||
    /busy|locked/i.test(String(error?.message ?? ''))
  );
}

/** Blocking sleep. A worker has no event loop to yield to mid-transaction. */
function sleep(ms) {
  const until = performance.now() + ms;
  while (performance.now() < until) {
    // spin
  }
}

const HANDLERS = { open, createSchema, write, read, verify, close };

self.onmessage = async (event) => {
  const { id, cmd, args } = event.data;
  try {
    const handler = HANDLERS[cmd];
    if (!handler) throw new Error(`unknown command: ${cmd}`);
    const result = await handler(args ?? {});
    self.postMessage({ id, ok: true, result });
  } catch (error) {
    self.postMessage({
      id,
      ok: false,
      error: `${label}: ${error?.message ?? error}`,
      code: error?.resultCode ?? null,
    });
  }
};
