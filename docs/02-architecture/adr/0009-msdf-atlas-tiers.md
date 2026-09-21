# ADR 0009 — Pre-bake one MSDF tier; load the rest on demand

**Status:** Accepted · 2026-09-21 · re-measure when the Phase 1 type audit fixes the faces

## Context

The canvas draws text with multi-channel signed distance fields, so that a name stays crisp from
the `z2` band to the `z3` card without re-rasterising. That requires the glyphs to be in a
texture, baked ahead of time. The roadmap set the budget:

> Full Russian and German coverage under 4 MB, with a working on-demand path for the rest.

Spike **S3** measured it.

## The measurement

`tools/measure-msdf.sh`, run 2026-09-21 with msdf-atlas-gen 1.4 at 48 px em and a 4 px distance
range, power-of-two atlases. Six instances — the set the *canvas* needs, which is far smaller
than the design system's, because interface chrome is DOM text and costs no atlas at all.

Two coverage tiers:

- **A** — ASCII, Russian, the German additions, and the typographic punctuation Russian text
  requires: «ёлочки», em and en dashes, №. **183 glyphs.**
- **B** — A plus Latin-1, Latin Extended-A, Greek and the full Cyrillic block. **392–569
  glyphs** depending on what the face actually contains.

| Instance | Tier A atlas | A png | A json | Tier B atlas | B png | B json |
|---|---|---|---|---|---|---|
| Source Serif 4 @ 400 | 512² | 204 KiB | 42 KiB | 1024² | 616 KiB | 133 KiB |
| Source Serif 4 @ 600 | 512² | 207 KiB | 42 KiB | 1024² | 721 KiB | 133 KiB |
| Golos Text @ 400 | 512² | 154 KiB | 43 KiB | 1024² | 413 KiB | 113 KiB |
| Golos Text @ 600 | 512² | 157 KiB | 43 KiB | 1024² | 441 KiB | 113 KiB |
| JetBrains Mono @ 400 | 512² | 131 KiB | 42 KiB | 1024² | 361 KiB | 119 KiB |
| Prata @ 400 | 512² | 200 KiB | 42 KiB | 1024² | 419 KiB | 91 KiB |
| **Total** | | **1 054 KiB** | **254 KiB** | | **2 970 KiB** | **702 KiB** |

**Tier A: 1 308 KiB — passes at roughly a third of budget. Tier B: 3 672 KiB — fits, with
almost nothing to spare.**

## Decision

1. **Pre-bake tier A.** ASCII, Russian, German and typographic punctuation ship with the
   application.
2. **Load tier B on demand, per Unicode block.** Greek, Latin Extended-A and the rarer Cyrillic
   arrive when a tree actually contains them.
3. **Ship glyph metrics as packed binary, not JSON.** See below.
4. **Atlases are 512² wherever possible.** Crossing to 1024² is a deliberate act, not a
   consequence of adding glyphs carelessly.

## Consequences

**Gained.** A 1.3 MB text payload, three times under budget, at 512² — the smallest practical
atlas. VRAM matters as much as download here: six 512² RGB atlases are **4.6 MB** resident,
where six at 1024² would be **18 MB**. On a mid-range phone, already running our WASM core and a
WebGL canvas, that difference is the one that decides whether the app is comfortable.

**Given up.** Instant Greek. A tree citing a Greek-language source shows a brief glyph load the
first time. Acceptable: the on-demand path was a budgeted requirement anyway, and this is exactly
what it is for.

### Three findings worth recording

**The metrics are 19–24 % of the payload, and should not be.** 254 KiB of JSON describes 1 098
glyph records — position, size, bearing, advance. Packed binary at 32 bytes per glyph is about
**36 KiB for the whole set**, an 86 % saving on that component and 17 % on the total. JSON is the
tool's output format, not a reason to ship it.

**Every weight costs a full atlas.** Source Serif 4 at 400 and at 600 are 204 and 207 KiB — the
atlas grid is set by glyph count and cell size, not by how much ink is in each cell. So the
weight count multiplies directly, and the canvas must justify each one. Six instances is already
a decision to keep the canvas typographically simple; the design system's full weight range
stays in DOM text where it costs nothing. Heavier weights do compress slightly worse — tier B at
600 is 17 % larger than at 400 — which is the ink, not the grid.

**msdf-atlas-gen 1.4 silently ignores `-varfont` axis settings.** Atlases baked at `wght=200`
and `wght=900` came out **byte-identical**. Had that gone unnoticed, this ADR would have
reported six distinct weights that were in fact one weight repeated. The script now instantiates
static weights with fontTools and **fails if any two instances hash the same** — a guard against
the specific way this measurement can lie.

## Alternatives considered

- **Bake tier B and be done.** It fits, at 3.67 MB of 4 MB, and it costs 1024² atlases and 18 MB
  of VRAM for coverage most trees never touch. Declined on the memory, not the download.
- **32 px instead of 48 px.** Roughly 30 % smaller. Not taken for now: the `z3` card renders
  names large, and distance-field quality at high magnification is the thing MSDF is *for*.
  Worth revisiting with a visual comparison in Phase 1, when there is something to compare on.
- **One shared atlas across instances.** Fewer texture binds, at the cost of rebaking everything
  when one face changes. Premature until the type audit settles the faces.
- **Runtime rasterisation instead of pre-baking.** Simpler to ship, and it puts glyph
  rasterisation on the frame budget of a canvas already trying to hold 60 fps at 50 000 nodes.

## Reproducing

```sh
tools/measure-msdf.sh <path-to-msdf-atlas-gen> <font-dir> [out-dir]
```

Requires msdf-atlas-gen 1.4+, Python with fontTools, and Node. The measured faces are the
current candidates, not final: **the Phase 1 type audit still has to resolve that Archivo — named
as the data face in the art direction — has no Cyrillic**, which is why Golos Text stands in
here. Re-run this when that is settled.
