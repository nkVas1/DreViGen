#!/usr/bin/env node
// Generates crates/drevigen-gedcom/src/encoding/tables.rs: the character tables the GEDCOM reader
// decodes old files with.
//
// Every table comes from the body that defines it, fetched from a pinned location and checked
// against a recorded SHA-256, so a source that changes cannot slip in unnoticed. Nothing here is
// typed from memory except GEDCOM's own additions to ANSEL, which are quoted below with the
// appendix they come from. The sources and their licences are listed in
// crates/drevigen-gedcom/NOTICES.md.
//
// Run from anywhere:
//
//     node tools/generate-encoding-tables.mjs
//
// Sources are cached in target/encoding-sources/; delete it to fetch them again.

import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const cache = path.join(root, 'target', 'encoding-sources');
const output = path.join(root, 'crates', 'drevigen-gedcom', 'src', 'encoding', 'tables.rs');

const MAPPINGS = 'https://www.unicode.org/Public/MAPPINGS/VENDORS/MICSFT';
const UCD = 'https://www.unicode.org/Public/16.0.0/ucd';
const PYMARC = '738b200fc5218c03c56508e9b7cb40f9ca98b9c3';

const SOURCES = {
  'CP1250.TXT': [`${MAPPINGS}/WINDOWS/CP1250.TXT`, 'e6535e3c81f4aff1a4b369c46e588a2423d6a75da341e01a6cea108c7542c19a'],
  'CP1251.TXT': [`${MAPPINGS}/WINDOWS/CP1251.TXT`, 'd9d491808bd7956c26ef8ed07fa63015bfab32860720e07d5bf3a64a609927ee'],
  'CP1252.TXT': [`${MAPPINGS}/WINDOWS/CP1252.TXT`, 'f607ae328b4dff5e9bfef725f5fff0ae23f38797f8a5b95998a0d2735c0e8fad'],
  'CP437.TXT': [`${MAPPINGS}/PC/CP437.TXT`, '6bad4dabcdf5940227c7d81fab130dcb18a77850b5d79de28b5dc4e047b0aaac'],
  'CP850.TXT': [`${MAPPINGS}/PC/CP850.TXT`, 'ffdcc3c1c72f1aef600a63547100ef3dc452a09ad84923d382085519751c7479'],
  'CP866.TXT': [`${MAPPINGS}/PC/CP866.TXT`, 'abcc96dd4253321eb5e542c1ece3adab10df0cc20ec5d1124a0cec22d636c924'],
  // Mac OS Roman from the WHATWG Encoding Standard rather than Apple's own table: the two agree
  // byte for byte, and WHATWG's is published under CC BY 4.0 where Apple's reserves all rights.
  'index-macintosh.txt': ['https://encoding.spec.whatwg.org/index-macintosh.txt', '5e2b0aa162f3032cf6c96a119f59f698da80c16d58014080a6bd246d7ade3c38'],
  'UnicodeData.txt': [`${UCD}/UnicodeData.txt`, 'ff58e5823bd095166564a006e47d111130813dcf8bf234ef79fa51a870edb48f'],
  'CompositionExclusions.txt': [`${UCD}/CompositionExclusions.txt`, '89e83cf9cc8bef6c1f8bf77e42cf6f0341dfa42e66261f4dbe9b492e7a23c8ee'],
  // The Library of Congress's MARC-8 tables, as pymarc transcribes them. The Library's own page
  // (loc.gov/marc/specifications/codetables.xml) is the primary source but does not answer
  // reliably; pymarc's transcription is the one most MARC software has been tested against.
  'marc8_mapping.py': [`https://gitlab.com/pymarc/pymarc/-/raw/${PYMARC}/pymarc/marc8_mapping.py`, 'fda2ffacd0c612a86016ec77ecd8b341ad436961750f43fa1ee1fca0b201b015'],
};

// What GEDCOM's ANSEL adds to MARC-8's Extended Latin set: the characters GEDCOM 5.5's Appendix C
// marks "LDS extension", and the es-zet that 5.5.1's Appendix C still lists. Read from the
// appendices, not recalled.
const GEDCOM_ADDITIONS = [
  [0xbe, 0x25a1, 'empty box: WHITE SQUARE'],
  [0xbf, 0x25a0, 'black box: BLACK SQUARE'],
  [0xcd, 0x0065, 'e in middle of line: the letter; its raised position has no Unicode form'],
  [0xce, 0x006f, 'o in middle of line: likewise'],
  [0xcf, 0x00df, 'es zet, as in "Preußen"'],
  [0xfc, 0x0338, 'diacritic slash through char: COMBINING LONG SOLIDUS OVERLAY'],
];

// MARC's non-sort markers, which bracket a leading article in a library title. They decode to C1
// controls, which GEDCOM 7 bans, and GEDCOM never defined them; a file containing them gets them
// reported, not decoded.
const MARC_ONLY = [0x88, 0x89];

async function source(name) {
  const [url, expected] = SOURCES[name];
  const file = path.join(cache, name);
  let bytes;
  try {
    bytes = await readFile(file);
  } catch {
    bytes = await download(url);
    await mkdir(cache, { recursive: true });
    await writeFile(file, bytes);
  }
  const actual = createHash('sha256').update(bytes).digest('hex');
  if (actual !== expected) {
    throw new Error(`${name}: SHA-256 is ${actual}, expected ${expected}. The source changed; review it before updating the hash.`);
  }
  return bytes.toString('utf8');
}

async function download(url) {
  // unicode.org drops the occasional TLS handshake; a few attempts ride that out.
  for (let attempt = 1; ; attempt++) {
    try {
      const response = await fetch(url, { signal: AbortSignal.timeout(90_000) });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      return Buffer.from(await response.arrayBuffer());
    } catch (error) {
      if (attempt === 5) throw new Error(`could not fetch ${url}: ${error.cause ?? error}`);
      await new Promise((resolve) => setTimeout(resolve, 3000));
    }
  }
}

/** The upper half of a Unicode-format code page table: 128 code points, 0 where undefined. */
function codePage(text) {
  const table = new Array(128).fill(0);
  for (const line of text.split('\n')) {
    const match = /^0x([0-9A-F]{2})\s+0x([0-9A-F]{4})/i.exec(line);
    if (!match) continue;
    const byte = parseInt(match[1], 16);
    if (byte >= 0x80) table[byte - 0x80] = parseInt(match[2], 16);
  }
  return table;
}

/** A WHATWG single-byte index: pointer 0–127 stands for byte 0x80–0xFF. */
function whatwgIndex(text) {
  const table = new Array(128).fill(0);
  for (const line of text.split('\n')) {
    const match = /^\s*(\d+)\s+0x([0-9A-F]{4})/i.exec(line);
    if (match) table[Number(match[1])] = parseInt(match[2], 16);
  }
  return table;
}

/** GEDCOM's ANSEL: the upper half, and which of its bytes are combining marks. */
function ansel(marc8) {
  const start = marc8.indexOf('CHARSET_45 = {');
  const end = marc8.indexOf('\n}', start);
  if (start < 0 || end < 0) throw new Error('CHARSET_45 not found in marc8_mapping.py');
  const block = marc8
    .slice(start, end)
    .split('\n')
    .map((line) => line.replace(/#.*$/, ''))
    .join(' ');

  const table = new Array(128).fill(0);
  const marks = new Set();
  for (const match of block.matchAll(/0x([0-9A-F]{2}):\s*\(\s*0x([0-9A-F]+)\s*,\s*([01])\s*,?\s*\)/gi)) {
    const byte = parseInt(match[1], 16);
    if (MARC_ONLY.includes(byte)) continue;
    table[byte - 0x80] = parseInt(match[2], 16);
    if (match[3] === '1') marks.add(byte);
  }
  for (const [byte, code] of GEDCOM_ADDITIONS) {
    if (table[byte - 0x80] !== 0) throw new Error(`0x${byte.toString(16)} is already defined by MARC-8`);
    table[byte - 0x80] = code;
    if (byte >= 0xe0) marks.add(byte);
  }

  // The decoder treats exactly 0xE0–0xFE as marks; check the sources agree.
  for (let byte = 0x80; byte <= 0xff; byte++) {
    const defined = table[byte - 0x80] !== 0;
    const mark = marks.has(byte);
    if (mark && byte < 0xe0) throw new Error(`0x${byte.toString(16)} is a mark outside 0xE0–0xFE`);
    if (defined && byte >= 0xe0 && !mark) throw new Error(`0x${byte.toString(16)} is a spacing character inside 0xE0–0xFE`);
  }
  return { table, marks: [...marks].sort((a, b) => a - b).map((byte) => table[byte - 0x80]) };
}

/**
 * The canonical compositions an ANSEL decoder can need: every pair (starter, mark) → composite
 * where the mark is one ANSEL has and the starter is something ANSEL can produce — a printable
 * ASCII character, an ANSEL spacing character, or a composite already reachable that way.
 */
function compositions(unicodeData, exclusionsText, marks, spacing) {
  const combiningClass = new Map();
  const pairs = [];
  for (const line of unicodeData.split('\n')) {
    const fields = line.split(';');
    if (fields.length < 6) continue;
    const code = parseInt(fields[0], 16);
    const ccc = Number(fields[3]);
    if (ccc !== 0) combiningClass.set(code, ccc);
    const decomposition = fields[5];
    if (decomposition && !decomposition.startsWith('<')) {
      const parts = decomposition.split(' ').map((part) => parseInt(part, 16));
      if (parts.length === 2) pairs.push([parts[0], parts[1], code]);
    }
  }

  const excluded = new Set();
  for (const line of exclusionsText.split('\n')) {
    const match = /^([0-9A-F]{4,6})(?:\.\.([0-9A-F]{4,6}))?/i.exec(line);
    if (!match) continue;
    const first = parseInt(match[1], 16);
    const last = match[2] ? parseInt(match[2], 16) : first;
    for (let code = first; code <= last; code++) excluded.add(code);
  }

  // UAX #15: a primary composite is a canonical decomposition that is not excluded and does not
  // start with a non-starter, and whose result is itself a starter.
  const primary = pairs.filter(
    ([starter, , composite]) =>
      !excluded.has(composite) && !combiningClass.has(starter) && !combiningClass.has(composite),
  );

  const markSet = new Set(marks);
  const reachable = new Set(spacing);
  for (let code = 0x20; code < 0x7f; code++) reachable.add(code);
  for (let grew = true; grew; ) {
    grew = false;
    for (const [starter, mark, composite] of primary) {
      if (reachable.has(starter) && markSet.has(mark) && !reachable.has(composite)) {
        reachable.add(composite);
        grew = true;
      }
    }
  }

  const table = primary
    .filter(([starter, mark]) => reachable.has(starter) && markSet.has(mark))
    .sort((a, b) => a[0] - b[0] || a[1] - b[1]);
  for (const code of [...marks, ...table.flat()]) {
    if (code > 0xffff) throw new Error(`U+${code.toString(16)} does not fit the u16 tables`);
  }
  for (const mark of marks) {
    if (!combiningClass.has(mark)) throw new Error(`U+${mark.toString(16)} has no combining class`);
  }
  return { table, classes: marks.map((mark) => [mark, combiningClass.get(mark)]).sort((a, b) => a[0] - b[0]) };
}

const hex = (code) => `0x${code.toString(16).toUpperCase().padStart(4, '0')}`;

function upperHalf(name, doc, table) {
  const rows = [];
  for (let row = 0; row < 128; row += 8) {
    rows.push(`    /* ${hex(0x80 + row).slice(4)} */ ${table.slice(row, row + 8).map(hex).join(', ')},`);
  }
  return `${doc}\n#[rustfmt::skip]\npub(super) const ${name}: [u16; 128] = [\n    // The comment on each row is its first byte, 0x80–0xF8.\n${rows.join('\n')}\n];\n`;
}

async function main() {
  const marc8 = await source('marc8_mapping.py');
  const { table: anselTable, marks } = ansel(marc8);
  const spacing = anselTable.filter((code, index) => code !== 0 && index < 0x60);
  const { table: composition, classes } = compositions(
    await source('UnicodeData.txt'),
    await source('CompositionExclusions.txt'),
    marks,
    spacing,
  );

  const pages = [
    ['WINDOWS_1250', 'windows-1250, Central European', codePage(await source('CP1250.TXT'))],
    ['WINDOWS_1251', 'windows-1251, Cyrillic', codePage(await source('CP1251.TXT'))],
    ['WINDOWS_1252', 'windows-1252, Western European', codePage(await source('CP1252.TXT'))],
    ['IBM437', 'IBM437, the original IBM PC code page', codePage(await source('CP437.TXT'))],
    ['IBM850', 'IBM850, DOS Western European', codePage(await source('CP850.TXT'))],
    ['IBM866', 'IBM866, DOS Cyrillic', codePage(await source('CP866.TXT'))],
    ['MACINTOSH', 'Mac OS Roman', whatwgIndex(await source('index-macintosh.txt'))],
  ];

  const parts = [
    `// Generated by tools/generate-encoding-tables.mjs from the sources listed in
// crates/drevigen-gedcom/NOTICES.md, which also carries their licences. Do not edit; regenerate.

//! The character tables: what each byte from 0x80 up means in each legacy encoding, and the
//! canonical compositions the ANSEL decoder needs to produce composed text.
//!
//! Every encoding here keeps 0x00–0x7F as ASCII, so each table holds only the upper half: entry
//! \`n\` is the code point of byte \`0x80 + n\`, and \`0\` means the byte stands for nothing.
`,
  ];
  for (const [name, title, table] of pages) {
    parts.push(upperHalf(name, `/// ${title}.`, table));
  }
  parts.push(
    upperHalf(
      'ANSEL',
      "/// GEDCOM's ANSEL: MARC-8 Extended Latin and GEDCOM's additions to it. The bytes 0xE0–0xFE are\n/// combining marks, written before the letter they belong to.",
      anselTable,
    ),
  );
  parts.push(
    `/// The canonical combining class of each mark ANSEL can produce, sorted by mark.
#[rustfmt::skip]
pub(super) const COMBINING_CLASSES: [(u16, u8); ${classes.length}] = [
${classes.map(([mark, ccc]) => `    (${hex(mark)}, ${ccc}),`).join('\n')}
];
`,
  );
  parts.push(
    `/// Every primary composite reachable from ANSEL: \`(starter, mark, composite)\`, sorted by
/// starter and then mark. From the Unicode Character Database 16.0.0, composition exclusions
/// removed.
#[rustfmt::skip]
pub(super) const COMPOSITIONS: [(u16, u16, u16); ${composition.length}] = [
${composition.map(([starter, mark, composite]) => `    (${hex(starter)}, ${hex(mark)}, ${hex(composite)}),`).join('\n')}
];
`,
  );

  await mkdir(path.dirname(output), { recursive: true });
  await writeFile(output, parts.join('\n'));
  console.log(`Wrote ${path.relative(root, output)}: 8 tables, ${classes.length} marks, ${composition.length} compositions.`);
}

await main();
