// Static server for spike S4.
//
// It exists for one reason the usual `python -m http.server` cannot satisfy: SQLite's `opfs`
// VFS is built on SharedArrayBuffer, which browsers only expose to a **cross-origin isolated**
// document. That means two headers, and without them the VFS is simply absent — silently, with
// the page falling back to an in-memory database that passes every test and persists nothing.
//
//   node server.mjs [port]

import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import { extname, join, normalize, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = resolve(fileURLToPath(new URL('.', import.meta.url)));
const PORT = Number(process.argv[2] ?? 8732);

const TYPES = new Map(
  Object.entries({
    '.html': 'text/html; charset=utf-8',
    '.js': 'text/javascript; charset=utf-8',
    '.mjs': 'text/javascript; charset=utf-8',
    '.json': 'application/json; charset=utf-8',
    '.wasm': 'application/wasm',
    '.css': 'text/css; charset=utf-8',
  }),
);

const server = createServer(async (req, res) => {
  // Cross-origin isolation. Without both of these, SharedArrayBuffer is undefined and the opfs
  // VFS never registers.
  res.setHeader('Cross-Origin-Opener-Policy', 'same-origin');
  res.setHeader('Cross-Origin-Embedder-Policy', 'require-corp');
  res.setHeader('Cross-Origin-Resource-Policy', 'same-origin');
  res.setHeader('Cache-Control', 'no-store');

  const url = new URL(req.url ?? '/', `http://localhost:${PORT}`);
  const requested = url.pathname === '/' ? '/index.html' : url.pathname;

  // Resolve inside ROOT only. A spike is still a server.
  const target = resolve(join(ROOT, normalize(decodeURIComponent(requested))));
  if (!target.startsWith(ROOT)) {
    res.writeHead(403).end('outside root');
    return;
  }

  try {
    const info = await stat(target);
    if (!info.isFile()) throw new Error('not a file');
    const body = await readFile(target);
    res.writeHead(200, {
      'Content-Type': TYPES.get(extname(target)) ?? 'application/octet-stream',
      'Content-Length': body.byteLength,
    });
    res.end(body);
  } catch {
    res.writeHead(404).end(`not found: ${requested}`);
  }
});

server.listen(PORT, '127.0.0.1', () => {
  console.log(`S4 harness on http://127.0.0.1:${PORT}/ (cross-origin isolated)`);
});
