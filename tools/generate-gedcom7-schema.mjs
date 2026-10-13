#!/usr/bin/env node
// Generates crates/drevigen-gedcom/src/schema/tables.rs: GEDCOM 7's structure rules — which
// structures may appear where, how many times, and with what payload — as tables.
//
// FamilySearch generates these rules from the specification source and publishes them beside it,
// in `extracted-files/`. They are fetched at the commit the rest of this crate was checked
// against, and verified against recorded SHA-256 digests, so a change upstream cannot slip in
// unnoticed. Nothing here is typed from memory. The licence and notice are in
// crates/drevigen-gedcom/NOTICES.md.
//
// Run from anywhere:
//
//     node tools/generate-gedcom7-schema.mjs
//
// Sources are cached in target/gedcom7-schema/; delete it to fetch them again.

import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const cache = path.join(root, 'target', 'gedcom7-schema');
const output = path.join(root, 'crates', 'drevigen-gedcom', 'src', 'schema', 'tables.rs');

const COMMIT = '512e38d0cd8b882f9cd23256e53df78a06fe702d';
const BASE = `https://raw.githubusercontent.com/FamilySearch/GEDCOM/${COMMIT}/extracted-files`;
const G7 = 'https://gedcom.io/terms/v7/';

const TABLES = {
  'substructures.tsv': '90501ba37b3c4c6140f6da7b8054303c08ffbb1809b1ea058e61df422743f3b1',
  'cardinalities.tsv': 'cf665f5cd5d3115bdcfce9ead209d6921d85336e502a3489092fc09b981c97be',
  'payloads.tsv': 'f0a8b3ac222ed7ae0736aebf87911218278ad6e4f755449ee44090cec3b8c1e5',
  'enumerations.tsv': '93a2e168e3d0aed5ccb58b0caeae224684dc48070d2cfbacf34733d3960cf999',
  'enumerationsets.tsv': '1f898fc753a9156b044ef678ed324b865a0a451f164564bf35c04f5fe5250b86',
};

// The tables name enumeration values by URI only; the tag a file writes for each is in the
// value's own YAML file under `tags/`. Those files are checked together: one digest of their
// contents, in the order of their URIs sorted.
const TAGS_DIGEST = '33dea483b95679b868cc150505836edbce0546b51396731f76629db736e188a2';

// The payload types the tables use, and what this crate calls them. A type not listed here stops
// the generator: a new data type needs a grammar before it needs a table entry.
const DATA_TYPES = {
  '': 'None',
  'http://www.w3.org/2001/XMLSchema#string': 'Text',
  'Y|<NULL>': 'Y',
  'https://gedcom.io/terms/v7/type-Enum': 'Enum',
  'https://gedcom.io/terms/v7/type-List#Enum': 'EnumList',
  'https://gedcom.io/terms/v7/type-List#Text': 'TextList',
  'http://www.w3.org/2001/XMLSchema#nonNegativeInteger': 'Integer',
  'https://gedcom.io/terms/v7/type-Date': 'Date',
  'https://gedcom.io/terms/v7/type-Date#exact': 'DateExact',
  'https://gedcom.io/terms/v7/type-Date#period': 'DatePeriod',
  'https://gedcom.io/terms/v7/type-Time': 'Time',
  'https://gedcom.io/terms/v7/type-Age': 'Age',
  'https://gedcom.io/terms/v7/type-Name': 'PersonalName',
  'http://www.w3.org/2001/XMLSchema#Language': 'Language',
  'http://www.w3.org/ns/dcat#mediaType': 'MediaType',
  'https://gedcom.io/terms/v7/type-FilePath': 'FilePath',
  'http://www.w3.org/2001/XMLSchema#anyURI': 'Uri',
  'https://gedcom.io/terms/v7/type-TagDef': 'TagDefinition',
  'https://gedcom.io/terms/v7/type-Latitude': 'Latitude',
  'https://gedcom.io/terms/v7/type-Longitude': 'Longitude',
};

const CARDINALITIES = {
  '{0:1}': '{ required: false, many: false }',
  '{1:1}': '{ required: true, many: false }',
  '{0:M}': '{ required: false, many: true }',
  '{1:M}': '{ required: true, many: true }',
};

async function cached(name, url) {
  const file = path.join(cache, name);
  try {
    return await readFile(file);
  } catch {
    const bytes = await download(url);
    await mkdir(path.dirname(file), { recursive: true });
    await writeFile(file, bytes);
    return bytes;
  }
}

async function download(url) {
  // raw.githubusercontent.com drops the occasional connection; a few attempts ride that out.
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

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

async function table(name) {
  const bytes = await cached(name, `${BASE}/${name}`);
  const actual = sha256(bytes);
  if (actual !== TABLES[name]) {
    throw new Error(`${name}: SHA-256 is ${actual}, expected ${TABLES[name]}. The source changed; review it before updating the digest.`);
  }
  const [, ...rows] = bytes.toString('utf8').split('\n').filter((line) => line.length > 0);
  return rows.map((row) => row.split('\t'));
}

/** A GEDCOM 7 URI as this crate names it: what follows the namespace. */
function id(uri) {
  if (!uri.startsWith(G7)) throw new Error(`not a GEDCOM 7 URI: ${uri}`);
  return uri.slice(G7.length);
}

function compare(a, b) {
  for (let i = 0; i < a.length; i++) {
    if (a[i] < b[i]) return -1;
    if (a[i] > b[i]) return 1;
  }
  return 0;
}

const quote = (text) => JSON.stringify(text);

async function main() {
  const substructures = (await table('substructures.tsv')).map(([within, tag, structure]) => [
    within === '' ? '' : id(within),
    tag,
    id(structure),
  ]);
  const seen = new Set();
  for (const [within, tag] of substructures) {
    const key = `${within} ${tag}`;
    if (seen.has(key)) throw new Error(`${tag} means two structures within ${within || 'the dataset'}`);
    seen.add(key);
  }
  const recordTags = new Map(substructures.filter(([within]) => within === '').map(([, tag, structure]) => [structure, tag]));
  const tagOf = new Map(substructures.map(([within, tag, structure]) => [`${within} ${structure}`, tag]));

  const cardinalities = (await table('cardinalities.tsv')).map(([within, structure, cardinality]) => {
    const rust = CARDINALITIES[cardinality];
    if (!rust) throw new Error(`unknown cardinality ${cardinality}`);
    const tag = tagOf.get(`${id(within)} ${id(structure)}`);
    if (!tag) throw new Error(`${structure} has a cardinality within ${within} but no tag there`);
    return [id(within), id(structure), tag, rust];
  });

  const payloads = (await table('payloads.tsv')).map(([structure, payload = '']) => {
    const pointer = /^@<(.+)>@$/.exec(payload);
    if (pointer) {
      const record = recordTags.get(id(pointer[1]));
      if (!record) throw new Error(`${structure} points at ${pointer[1]}, which is not a record`);
      return [id(structure), `DataType::Pointer(${quote(record)})`];
    }
    const name = DATA_TYPES[payload];
    if (!name) throw new Error(`${structure}: no grammar for payload type ${payload}`);
    return [id(structure), `DataType::${name}`];
  });
  const typed = new Set(payloads.map(([structure]) => structure));
  for (const [, , structure] of substructures) {
    if (!typed.has(structure)) throw new Error(`${structure} has no payload type`);
  }

  const enumerations = (await table('enumerations.tsv')).map(([structure, set]) => [id(structure), id(set)]);
  const members = (await table('enumerationsets.tsv')).map(([set, value]) => [id(set), value]);

  const values = [...new Set(members.map(([, value]) => value))].sort();
  const texts = [];
  const standardTags = new Map();
  for (const value of values) {
    const text = (await cached(path.join('tags', id(value)), `${BASE}/tags/${id(value)}`)).toString('utf8');
    texts.push(text);
    const match = /^standard tag:\s*'?([^'\n]+?)'?\s*$/m.exec(text);
    if (!match) throw new Error(`${value} has no standard tag`);
    standardTags.set(value, match[1]);
  }
  const digest = sha256(texts.join(''));
  if (digest !== TAGS_DIGEST) {
    throw new Error(`the enumeration values' YAML files digest to ${digest}, expected ${TAGS_DIGEST}. Review them before updating the digest.`);
  }
  const setValues = members.map(([set, value]) => [set, standardTags.get(value)]);
  const sets = new Set(setValues.map(([set]) => set));
  for (const [structure, set] of enumerations) {
    if (!sets.has(set)) throw new Error(`${structure} draws from ${set}, which has no values`);
  }

  substructures.sort(compare);
  cardinalities.sort(compare);
  payloads.sort(compare);
  enumerations.sort(compare);
  setValues.sort(compare);

  const source = `// Generated by tools/generate-gedcom7-schema.mjs from the files FamilySearch extracts from the
// GEDCOM 7 specification, at FamilySearch/GEDCOM@${COMMIT.slice(0, 7)}. See
// crates/drevigen-gedcom/NOTICES.md for their licence. Do not edit; regenerate.

//! GEDCOM 7's structure rules as the specification's own extracted files state them. A structure
//! type is named by its URI without the \`https://gedcom.io/terms/v7/\` namespace: \`record-INDI\`,
//! \`INDI-EVEN\`, \`DATE\`. Every table is sorted by its leading columns, for binary search.

use super::{Cardinality, DataType};

/// Where each standard tag may appear, and what structure it is there: \`(superstructure, tag,
/// structure)\`. A record's superstructure is the empty string.
#[rustfmt::skip]
pub(super) static SUBSTRUCTURES: [(&str, &str, &str); ${substructures.length}] = [
${substructures.map(([within, tag, structure]) => `    (${quote(within)}, ${quote(tag)}, ${quote(structure)}),`).join('\n')}
];

/// How many of each substructure a structure may have: \`(superstructure, structure, its tag there,
/// cardinality)\`.
#[rustfmt::skip]
pub(super) static CARDINALITIES: [(&str, &str, &str, Cardinality); ${cardinalities.length}] = [
${cardinalities.map(([within, structure, tag, cardinality]) => `    (${quote(within)}, ${quote(structure)}, ${quote(tag)}, Cardinality ${cardinality}),`).join('\n')}
];

/// What each structure's payload must be.
#[rustfmt::skip]
pub(super) static PAYLOADS: [(&str, DataType); ${payloads.length}] = [
${payloads.map(([structure, type]) => `    (${quote(structure)}, ${type}),`).join('\n')}
];

/// The enumeration set each structure with an enumeration payload draws its values from.
#[rustfmt::skip]
pub(super) static ENUMERATIONS: [(&str, &str); ${enumerations.length}] = [
${enumerations.map(([structure, set]) => `    (${quote(structure)}, ${quote(set)}),`).join('\n')}
];

/// The standard values of each enumeration set, as a file writes them: \`(set, tag)\`.
#[rustfmt::skip]
pub(super) static SET_VALUES: [(&str, &str); ${setValues.length}] = [
${setValues.map(([set, tag]) => `    (${quote(set)}, ${quote(tag)}),`).join('\n')}
];
`;
  await mkdir(path.dirname(output), { recursive: true });
  await writeFile(output, source);
  console.log(`Wrote ${path.relative(root, output)}: ${substructures.length} placements, ${cardinalities.length} cardinalities, ${payloads.length} payload types, ${setValues.length} enumeration values.`);
}

await main();
