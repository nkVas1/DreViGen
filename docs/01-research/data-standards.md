# Genealogical Data Standards

*Access date: 2026-09-21.*

## 1. FamilySearch GEDCOM 7 — the interchange format

GEDCOM 7.0 was released in 2021 after a two-decade freeze at 5.5.1. **The current revision is
7.0.18, dated 2026-02-17.** Adoption is real and broad — in Germany alone more than a dozen
vendors have shipped support. **[current]**

What 7.0 fixed that matters to us:

- **`.gdz` zip packaging** — the tree and its media travel as one file. This is the basis of our
  own container format.
- **Media may reference the internet as well as local files**, so a citation can point at a
  scanned image hosted by an archive.
- **Notes accept a defined subset of HTML** for basic rich text — so a research note can carry
  emphasis and structure without inventing a private markup.
- A precise, machine-readable specification with a registry of extension tags, ending the
  "every vendor invents `_CUSTOM` tags" era.

**GEDCOM X** exists in parallel with comparable expressive power. The division of labour in
practice: **GEDCOM 7 for single-researcher file exchange between applications; GEDCOM X for bulk
data transfer between services.** We implement GEDCOM 7 first and treat GEDCOM X as an
integration concern, not a storage concern. **[established]**

### Obligation

- Lossless GEDCOM 7.0.18 read **and** write, validated against the published specification.
- Tolerant GEDCOM 5.5/5.5.1 import, because that is what users actually have.
- Every construct we cannot represent must be preserved verbatim and round-tripped, not dropped.
  A documented loss report is shown on import.

### Existing parsers surveyed

| Library | Language | Note |
|---|---|---|
| `gedcom7code/js-gedcom` | JS | Parser, serializer, type-checker and validator, explicitly v7-focused, dependency-free |
| `arbre-app/read-gedcom` | TS | Tolerant parsing, full spec coverage, zero dependencies |
| `ge3224/ged_io` | Rust | Full-featured parser and writer with a type-safe API |

**[judgement]** None is a drop-in for our needs, because we need parse → *our* model →
serialize with byte-stability and a loss report. We will write our own parser in Rust, informed
by these three, and keep it in a standalone crate so it is independently useful. This is exactly
the case the project guidelines describe: borrow the ideas, integrate them properly.

## 2. The core object model

Adopted from Gramps with extensions. Objects are independent and reference one another by ID.

```
Person      ─┬─ has many  Name (with type: birth, married, religious, also-known-as)
             ├─ participates in  Event  via  EventRole
             ├─ is child in      Family
             ├─ is partner in    Family
             └─ has many  Attribute, Association, Media, Note, Tag

Family      ─── links up to two Partners + any number of Children,
                each child with a relationship type (birth, adopted, foster, step, …)

Event       ─── type + Date + Place + description; participants attach via roles
                (primary, spouse, witness, officiant, informant, godparent, …)

Place       ─── hierarchical (village ⊂ volost ⊂ uyezd ⊂ governorate),
                with coordinates, alternative and historical names, and validity periods

Source      ─── the document as a whole
Citation    ─── a specific location within a Source (page, entry, image, frame) + confidence
Repository  ─── where the Source physically or digitally lives (archive, fond, website)

Note, Media, Tag  ─── attachable to anything
```

Two consequences worth stating explicitly:

- **A marriage record with six named participants is one Event with six roles**, not two people
  with a `MARR` field and four orphaned notes.
- **A village is one Place object**, referenced by the two hundred events that happened there.
  Rename it once — in 1918, say — and every event keeps the name correct for its own date.

## 3. Places, dates and calendars

### Historical place names

Russian and Eastern European research makes this unavoidable: administrative geography changed
repeatedly, and a place has different correct names depending on the year of the record. Places
therefore carry **named periods** — a list of (name, authority, valid-from, valid-to) — rather
than one name. Display resolves the name against the date of the event being shown.

### Calendars

Research before 1918 in Russia (1752 in Britain, varying elsewhere) requires Julian/Gregorian
handling. The practical rules from genealogical practice: record the date **in the calendar the
source used**, mark which calendar that was, and let the software convert for display and
comparison. Double-dated originals (`23 Feb 1747/8`) must be storable as written. **[established]**

### Uncertain dates

Real genealogical dates are rarely points. The model must represent, natively:

| Form | Example |
|---|---|
| Exact | 1871-04-17 |
| Partial | 1871-04; 1871 |
| Range | between 1869 and 1873 |
| Bounded | before 1875; after 1869 |
| Approximate | about 1871; estimated 1871; calculated 1871 |
| Phrase | "in the third year after the fire" — unparseable but recordable |
| Dual-calendar | 1747/8 |

Arithmetic on these must be **interval arithmetic**, not point arithmetic. "Was this person alive
in 1900?" has three answers — yes, no, and *unknown* — and the software must be able to say the
third.

**Obligation.** A dedicated date type (Rust core, mirrored in TypeScript) implementing
GEDCOM 7 date grammar, Julian↔Gregorian↔Hebrew↔French-Republican conversion, interval
comparison, and a three-valued logic for temporal predicates. No general-purpose date library
covers this; we write it.

## 4. Assertions — our extension beyond Gramps

The gap in every tool surveyed: they store a researcher's **conclusion** and attach sources to
it. They cannot express *disagreement between sources* as data.

The Genealogical Proof Standard requires that contradictory evidence be **resolved**, and that
the resolution be **soundly reasoned and written**. That is impossible if the contradiction was
never representable in the first place.

Our model splits the layer:

```
Assertion  { subject, predicate, value, source_citation, confidence, asserted_by, asserted_at }
Conclusion { subject, predicate, value, supported_by[Assertion], contradicted_by[Assertion],
             reasoning, decided_by, decided_at }
```

- A birth-record image asserts `born = 1871-04-17`. A census asserts `born ≈ 1873`. Both are
  stored. Neither is deleted.
- The researcher writes a Conclusion preferring the birth record, citing the reasoning. The tree
  displays 1871; the conflict remains inspectable, one click away.
- If no Conclusion exists, the UI shows the conflict openly rather than picking silently. This
  is the **Открытый вопрос** state.
- Confidence follows the conventional four-level scale (very high / high / normal / low) already
  familiar from Gramps citations.

**[judgement]** This is our single most important structural differentiator. It costs real
complexity in the data layer and buys a tool that can actually support genealogical argument
rather than merely recording its output.

## 5. Names

Names are not strings, and Russian/Slavic research makes that impossible to ignore.

- **Structured parts:** given, patronymic, surname, prefix, suffix, nickname, call-name.
- **Multiple names per person,** each typed and dated: birth name, married name, religious name,
  transliterated name, name as written in a specific document.
- **Grammatical case** matters in Russian display ("родился у Ивана Петровича").
- **Transliteration** between Cyrillic and Latin is lossy and standard-dependent (GOST, ISO 9,
  BGN/PCGN, ALA-LC, passport-style). We store the original script and generate transliterations,
  never the reverse.

### Phonetic matching

For finding the same person across spellings, general-purpose Soundex is inadequate for Slavic
material. The relevant algorithms:

- **Daitch–Mokotoff Soundex** (1985) — a refinement of Russell/American Soundex built
  specifically for Slavic and Yiddish surnames with variant spellings. Six-digit codes;
  considers letter *sequences*, not just single letters; may emit multiple codes per name.
- **Beider–Morse Phonetic Matching** — infers the source language from spelling, then applies
  that language's phonetic rules. Cuts D–M's false-positive rate at the cost of some false
  negatives. Supports 16 languages including **Russian in both Latin and Cyrillic**.

**[established]** **Obligation.** Implement both, in Rust, with Beider–Morse as the default for
search and Daitch–Mokotoff as the wide net for match candidate generation.

## 6. Research apparatus (Genealogical Proof Standard)

The GPS, from the Board for Certification of Genealogists, has five elements:

1. Reasonably exhaustive research.
2. Complete and accurate source citation for every statement of fact.
3. Skilled correlation and interpretation of reliable evidence.
4. Resolution of contradictory evidence.
5. A soundly reasoned, coherently written conclusion.

Citation style follows Elizabeth Shown Mills, *Evidence Explained*, 4th ed. (2024) — the de facto
standard. **[established]**

**Obligation.** The Workshop must provide: a **research plan** (what question, which sources to
check), a **research log** (what was checked, including negative results — searched and *not*
found is evidence), **citation templates** following Evidence Explained patterns, a
**contradiction queue** listing unresolved conflicts, and **proof argument** documents that
compile into an exportable narrative.

Negative evidence deserves emphasis: no surveyed tool records "I searched this fond for these
years and she was not there", yet that is a substantial fraction of a genealogist's actual
labour and a requirement of element 1.

## 7. Privacy of living people

GDPR does not apply to processing "by a natural person in the course of a purely personal or
household activity" — which covers private family research — but **does** apply once data is
published or processed for research purposes at large. Contemporaries who request deletion of
their own data (or that of dependants) must be accommodated. **[established]**

**Obligation.**

- A computed **living** flag (no recorded death, born within a configurable threshold), with
  manual override.
- Per-tree privacy policy controlling what is exposed for living people at each visibility
  level, applied **at export and publish time as well as in the UI**.
- Field-level redaction that survives GEDCOM export and web publication.
- A right-to-erasure operation that genuinely removes a person from the canonical tree and all
  its published derivatives, distinct from ordinary tombstoning.

## 8. Summary of obligations

| Area | Deliverable |
|---|---|
| Interchange | Own GEDCOM 7.0.18 parser/serializer crate, lossless, with loss reporting |
| Core model | Gramps-derived object graph with event roles and source/citation split |
| Dates | Own date type: GEDCOM grammar, multi-calendar, intervals, three-valued predicates |
| Places | Hierarchical, coordinate-bearing, with dated name periods |
| Assertions | Assertion/Conclusion split with explicit contradiction state |
| Names | Structured, multi-instance, script-aware, with Daitch–Mokotoff + Beider–Morse matching |
| Research | Plan, log with negative results, Evidence Explained citation templates, proof arguments |
| Privacy | Living detection, per-level policy enforced at export, true erasure |
