# ADR 0011 — An import keeps everything: annotations, foreign structures, the file as a source

**Status:** Accepted · 2026-10-13

## Context

Phase 1 promises that a GEDCOM file imports and re-exports losslessly, and that whatever the model
cannot represent is preserved and reported rather than dropped. `drevigen-gedcom` now reads every
file into a forest of structures, whatever its version or encoding. Mapping that forest into the
archive meets four gaps in `drevigen-core`:

1. **Notes and media attach to nothing.** The archive holds `Note` and `Media` entities, but no
   person, family, event, source or fact can point at one — and in a real file nearly every record
   carries a `NOTE`, many an `OBJE`.
2. **Much of a file has no place in the model, and some never will.** Vendor extensions such as
   `_MILT` or `_UID`, `REFN`, `RIN`, `CHAN`, LDS ordinances, submitter records, a header's schema.
   Some of it will earn a place later; some is another program's private bookkeeping. All of it
   must come back out on export.
3. **Who says so?** ADR 0010 makes every value an assertion with an optional citation, and treats
   an uncited assertion as something a relative typed in from memory. A fact from a cousin's
   exported tree is neither memory nor primary evidence.
4. **Where does the code go?** The format crate was planned as standalone and useful outside
   DreViGen; mapping into the archive needs the archive.

## Decision

**1. The mapping lives in its own crate, `drevigen-interchange`.** It depends on
`drevigen-gedcom`, `drevigen-core` and `drevigen-date`, maps a document into an archive and back,
and assembles the import report. `drevigen-gedcom` stays what it is: bytes to structures and back,
with no dependency at all.

**2. Notes and media annotate anything.** The archive keeps links from any subject — an entity or
a fact, named by the existing `Entity` reference — to notes and to media. An inline GEDCOM `NOTE`
becomes a note annotating the structure it sat under; a shared note record becomes one note
annotating every subject that points at it. A note on a birth's date annotates the date fact, not
the event.

**3. What the model cannot represent is kept verbatim, on the nearest subject.** A `Foreign` item
holds the format it came from and the text as that format writes it — for GEDCOM, the structure
and everything beneath it as 7.0 lines, its own level written as 0 — and the subject it was found
under. Unknown records attach to the import's source (see 4). Foreign data is never interpreted,
merged with anything or shown as a fact; the dossier lists it as *kept from the import*, and the
exporter writes it back where it was found. When a later phase gives a structure a place in the
model, the importer maps it, and foreign data already stored stays foreign until someone converts
it — explicitly, as an edit with its own history.

**4. The file is a source.** Each import creates a `Source` for the file — its name, the program
and person that wrote it, the date in its header — and one `Citation` of that source per record,
its locator the record's identifier and line. Every imported value is an assertion made by the
person importing, at the moment of import:

- cited by each of the citations the file gives for it — a fact the file cites to three sources
  becomes three assertions of the same value, which ADR 0010 already reads as agreement;
- cited by the file's own citation when the file gives none, because *the tree said so* is a
  claim with a provenance, not a memory;
- with `QUAY` as its confidence, and never as a conclusion: importing makes no decisions.

**5. Identities are new.** Every record gets a fresh identity; the file's cross-reference
identifiers are used only while importing. `UID`, `_UID` and `EXID` are kept as foreign data until
re-importing a changed file — matching its records to those already imported — is designed.

**6. One report.** What decoding lost, what the reader repaired and what the mapping could not
place are listed together, each with its line and one of three severities — notation,
interpretation, loss — and shown to the person importing before they accept the result.

## Consequences

- `drevigen-core` gains two concepts, annotations and foreign items, that storage (1.4) and sync
  must carry. Both are plain data under ADR 0010: they are not claims.
- An archive imported from a cousin's file says so on every fact, which is what the provenance
  display needs, and costs one citation per record.
- Foreign data cannot be merged, only kept: two contributors' foreign items for one subject both
  survive, because nothing can tell which is right.
- The exporter must check foreign text before writing it. Text that came from a broken file may
  not be valid 7.0, and is then reported as not exported rather than written broken.
- When re-import matching is designed it will need the `UID`s kept now; nothing is lost by
  waiting.

## Alternatives considered

- **The mapping inside `drevigen-gedcom` behind a feature.** Couples the format crate to the
  domain model for no gain; the dependency arrow would point the wrong way.
- **A `notes`, `media` and `foreign` field on every entity and fact.** Facts are generic and shared
  by every value type; a field there touches every claim. Links keyed by subject cover all of them
  with one mechanism.
- **Dropping what cannot be mapped, with a report.** Violates the promise the roadmap makes, and
  the first export would destroy another program's data.
- **Keeping the original file beside the archive.** Faithful, but neither editable nor connected
  to anything in the archive. Worth storing as well, once media storage exists — as an attachment
  to the import's source, not instead of mapping.
- **Recording the file in the provenance rather than as a source.** Provenance answers *who
  recorded this and when*; *what document says so* is what a citation answers, and keeping the
  file a source lets the existing evidence apparatus — confidence, citation display, conflict
  resolution — treat a tree for what it is: a derivative source.
