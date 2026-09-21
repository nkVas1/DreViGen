# Visual Asset Production Brief

*For: Nikita, producing assets with an image-generation model.*
*Companion to [art-direction.md](./art-direction.md) — read that first; this document assumes it.*

---

## 0. Before you generate anything

### 0.1 What must **not** be generated

Three categories will look wrong if they come from an image model, and each has a better source.

| Category | Why not | Do this instead |
|---|---|---|
| **Icons and symbols** | They must be pixel-aligned, stroke-consistent, and legible at 16 px. Models cannot hold a 1 px stroke grid. | Draw as SVG on a 24 px grid with the five fixed line weights. Listed in §7 as a separate task. |
| **Tileable noise / grain** | A model cannot guarantee seamless tiling, and a visible seam across the whole app is fatal. | Generate procedurally — a script in `tools/textures/`. Spec in §2.1. |
| **Anything containing text** | Cyrillic in generated images is unreliable and will embarrass us. | All text is live type. Never bake a word into an asset. |

### 0.2 The universal style preamble

Prepend this to **every** prompt in this brief. It carries the art direction, so the individual
prompts only have to describe the subject.

> **STYLE PREAMBLE —**
> 19th-century scientific atlas illustration. Copperplate engraving and lithographic plate
> technique: fine hatching and cross-hatching, stippling for tone, confident varied line weight,
> no digital gradients, no airbrush, no glow, no bevel. Printed in iron-gall ink on warm vellum
> paper. Restrained natural palette — deep blue-black ink, terracotta red chalk (sanguine),
> muted sage green, warm sepia; low saturation throughout. Flat-on, centred, specimen-like
> composition with generous margins. The feel of a plate from a botanical or zoological atlas
> of the 1840s–1880s: precise, patient, hand-cut, slightly imperfect. Absolutely no 3D render,
> no photorealism, no neon, no gradient mesh, no glassmorphism, no stock-illustration flat
> vector style, no text or lettering anywhere in the image.

### 0.3 Universal negative prompt

> text, letters, words, watermark, signature, logo, 3D render, photorealistic, glossy, neon,
> gradient background, glassmorphism, drop shadow, flat vector corporate illustration, clip art,
> AI-art sheen, oversaturated, busy background, modern UI elements

### 0.4 Output discipline

- **Generate at 4× the largest use size**, then downsample. Models produce detail that only
  survives if you start big.
- **Deliver PNG with transparency** where the asset sits on the vellum ground. We convert to
  AVIF/WebP in the build.
- **File naming:** `<category>-<name>-<variant>@<scale>.png`, e.g. `ornament-corner-tl@4x.png`.
- **Drop everything in** `assets/source/<category>/`. The build pipeline in `tools/assets/`
  produces the optimised, correctly sized derivatives — never hand-resize.
- **Keep the prompt.** Every delivered file gets a sibling `.prompt.txt` with the exact prompt
  and seed used, so the asset can be regenerated or extended consistently later.

---

## 1. Identity

### A1 · App icon — the seed and the ring

The single most important asset. It appears on his home screen, in the Windows taskbar, and in
the macOS dock. It must be recognisable at 16 px and beautiful at 1024 px.

```
        SKETCH — concept: a seed in cross-section, whose
        growth rings are also generations

           ╭───────────────────────╮
           │     ·  ·  ·  ·  ·     │   ← outer ring: faint, many
           │   ·  ╭─────────╮  ·   │      (the distant generations)
           │  ·  ╱  ·  ·  ·  ╲  ·  │
           │  · │  ╭───────╮  │ ·  │   ← middle ring: engraved,
           │  · │ │   ╭─╮   │ │ ·  │      countable
           │  · │ │  │ ◆ │  │ │ ·  │   ← centre: a single filled
           │  · │ │   ╰─╯   │ │ ·  │      seed in sanguine
           │  · │  ╰───────╯  │ ·  │
           │  ·  ╲  ·  ·  ·  ╱  ·  │
           │   ·  ╰─────────╯  ·   │
           │     ·  ·  ·  ·  ·     │
           ╰───────────────────────╯

        Three concentric bands, hand-engraved, not perfectly
        circular. Radial hairlines connect centre to rim like
        the rays in a wood cross-section. Reads at 16 px as a
        dark ring with a warm dot at its heart.
```

**Prompt:** `[PREAMBLE] A botanical cross-section of a seed, viewed flat-on, rendered as a
copperplate engraving. Three concentric growth rings, each hand-cut with fine radial hatching
between them, the rings slightly irregular as in a real specimen. At the exact centre, one small
solid terracotta-red dot — the only colour in the image. Deep blue-black ink on warm cream
vellum. Perfectly symmetrical overall silhouette but organic in its internal line work. Isolated
on a plain vellum ground with wide margins. Iconic, simple enough to read at very small size.`

**Deliver:** 2048×2048 PNG, transparent background, plus a variant on a solid vellum square.
**Variants needed:** light ground, dark ground (ink becomes warm cream, dot stays terracotta),
monochrome single-colour silhouette for the Windows taskbar and macOS template icons.

### A2 · Adaptive icon layers (Android)

Same mark, split into **foreground** (the rings and seed, on transparency, safe zone 66 %) and
**background** (a plain vellum square with grain). 432×432 each.

### A3 · Social preview card

1280×640 for the GitHub repository card. The seed mark at left, generous vellum field, plate-mark
border. **No text in the image** — the wordmark is composited in the build from live type.

---

## 2. Grounds and textures

### 2.1 Paper grain — **generate procedurally, not with a model**

Specification for `tools/textures/generate-grain.ts`:

- **Tile:** 512×512, seamless (wrap-around noise).
- **Content:** three superimposed octaves of value noise — fine (1–2 px), medium (4–8 px), and a
  very low-frequency mottle (64 px+) for the uneven absorbency of handmade paper.
- **Laid lines:** optional horizontal chain lines every 24 px at 2 % opacity, and vertical laid
  lines every 1.2 px at 1 %.
- **Output:** 8-bit greyscale PNG, applied with `mix-blend-mode: multiply` at 4–7 % opacity.
- **Variants:** `grain-light`, `grain-dark` (inverted and warmed), `grain-print` (stronger, for
  export artefacts).

Verification: tile a 4000 px field and confirm no visible seam and no perceptible repeat rhythm.

### 2.2 Hatching and halftone patterns — procedural

Same pipeline. Cross-hatch at three densities (20 %, 40 %, 60 % coverage) at 45° and 135°;
halftone dot at three densities. Used for filled regions in charts and for photographic zones in
print export.

### 2.3 Foxing and age — **model-generated**, used sparingly

Sparse, irregular age spots for print-destined artefacts only. Never in working UI chrome.

**Prompt:** `[PREAMBLE] A sheet of aged cream vellum paper, flat-on, evenly lit, completely
blank. Scattered irregular pale brown foxing spots of varying size, concentrated toward the
edges, sparse in the centre. Subtle uneven discolouration. No creases, no folds, no shadows, no
objects. Pure paper surface only.`

**Deliver:** 3000×3000 PNG. We extract the spots to an alpha overlay in post.

### 2.4 Deckle edge and plate mark

For print and poster export only.

**Prompt:** `[PREAMBLE] A rectangular sheet of handmade cream vellum paper photographed flat-on
against pure white, showing its four natural deckle edges — soft, feathered, slightly irregular
torn edges. Inside the sheet, a faint rectangular plate mark impression about 4 cm in from each
edge, as left by a copperplate press. Completely blank paper otherwise. Even diffuse lighting,
no drop shadow.`

**Deliver:** 4000×5000 PNG. We cut it into a nine-slice frame.

---

## 3. Ornament

Used with severe restraint: title plates, print exports, the seal ceremony, empty states. Never
in working chrome.

### 3.1 Corner vignettes (set of 4)

```
        SKETCH — top-left corner ornament

        ╭─────────────────
        │ ⟋⟍
        │⟋   ⟍  ╱|
        │  ⟍  ╲╱ |     a single engraved branch with
        │    ⟍   |     3–5 leaves, curving inward from
        │      ╲ |     the corner, hairline-thin, no
        │       ╲|     fill, terminating in a small
        │        |     seed or bud
        │        |
```

**Prompt:** `[PREAMBLE] A single small botanical ornament for the top-left corner of a printed
page: one slender curving branch with three to five finely engraved leaves and one small closed
bud at its tip, drawn entirely in hairline copperplate outline with delicate hatching on the
leaf undersides. The branch enters from the upper-left corner and curves gently inward and down.
Asymmetric, restrained, elegant. Isolated on plain cream vellum with the rest of the frame
completely empty.`

**Deliver:** 4 files, 2000×2000 PNG transparent — the other three are *not* mirrors; generate
each so the set has hand-cut variety. Name `ornament-corner-{tl,tr,bl,br}@4x.png`.

### 3.2 Dividers and fleurons (set of 6)

**Prompt:** `[PREAMBLE] A horizontal typographic divider ornament in the style of a 19th-century
printed book: a slender symmetrical arrangement of engraved leaves, tendrils and a single small
central seed form, roughly eight times wider than it is tall, drawn in hairline outline with
fine hatching. Perfectly balanced left to right. Isolated, centred, on plain cream vellum.`

Generate six distinct ones. 3200×400 PNG transparent.

### 3.3 The seal — for the merged-contribution ceremony

The one ceremonial moment in the product: when a contribution is accepted into the canonical
tree, a seal presses onto the page.

```
        SKETCH

              ╭ ─ ─ ─ ─ ─ ─ ╮
           ╭╯   · · · · ·    ╰╮      circular, ~2 cm
          ╱   ·  ╭──────╮  ·   ╲     wax-seal impression,
         │   ·   │  ◆   │   ·   │    but engraved rather
         │   ·   │ ╱ ╲  │   ·   │    than photographic:
         │   ·   ╰──────╯   ·   │    the seed mark inside
          ╲   ·  · · · · ·  ╱       a laurel or wheat ring
           ╰╮              ╭╯
              ╰ ─ ─ ─ ─ ─ ╯
```

**Prompt:** `[PREAMBLE] A circular seal impression, engraved rather than photographed: a ring of
small wheat ears or laurel leaves encircling a simple central emblem of a seed in cross-section.
Rendered as a hand-cut stamp with slightly uneven, broken edges as if pressed into paper. Deep
terracotta-red ink, single colour, no shading beyond line work. Isolated on plain cream vellum,
centred.`

**Deliver:** 2000×2000 PNG transparent, plus a second "imperfect press" variant with one edge
lighter, so the animation can alternate and never look mechanical.

---

## 4. Illustration — empty states and onboarding

These carry the emotional tone. Each must work at roughly 480×360 in the UI.

### 4.1 Empty tree — "у вас пока никого нет"

```
        SKETCH

              ·
             ╱|╲          a single seedling: two
            ╱ | ╲         cotyledon leaves and a
           ╱  |  ╲        fine root system below
        ──────┼──────     the soil line, engraved
          ╲ ╱ | ╲ ╱       like a botanical plate.
           ╲  |  ╱        Lots of empty vellum
            ╲ | ╱         around it.
             ╲|╱
```

**Prompt:** `[PREAMBLE] A single young seedling with two simple cotyledon leaves, drawn as a
botanical specimen plate. The soil line is indicated by one horizontal hairline rule; below it,
a delicate branching root system is fully visible, drawn with the same care as the leaves above.
The plant is small and centred with a great deal of empty vellum around it. Hopeful rather than
sad. Hairline engraving with light hatching.`

### 4.2 Nothing found — "ничего не найдено"

**Prompt:** `[PREAMBLE] An engraved illustration of an empty specimen frame or pressing sheet: a
rectangular sheet of paper with four small paper corner-mounts, but no specimen mounted in it.
A magnifying glass rests beside it at an angle, drawn in the same hairline engraving style.
Quiet, patient, not comic. Centred on plain cream vellum with generous margins.`

### 4.3 No sources yet — "факт без источника"

**Prompt:** `[PREAMBLE] An engraved still life of a small stack of three closed archive folders
tied with a ribbon, seen at a slight angle, with one empty paper label on the topmost folder.
Hairline copperplate engraving, fine hatching on the folder edges. Isolated on cream vellum.`

### 4.4 Offline — "нет связи"

**Prompt:** `[PREAMBLE] An engraved illustration of a pressed flower specimen sealed between two
sheets of glass, resting on a desk — intact, preserved, waiting. Nothing broken or alarming.
Hairline engraving, fine hatching, cream vellum ground.`

### 4.5 Onboarding sequence (4 plates)

A four-step narrative, consistent in composition so they read as one series.

| # | Subject | Prompt subject line |
|---|---|---|
| 1 | **Начните с себя** | `A single pressed specimen mounted at the centre of an otherwise empty herbarium sheet, with one small blank paper label beneath it.` |
| 2 | **Добавьте родителей** | `The same herbarium sheet, now with two further pressed specimens mounted above the first and connected to it by two fine hand-ruled lines.` |
| 3 | **Приложите документы** | `A herbarium sheet with a specimen, beside which lie two small folded archive documents and a magnifying glass, all engraved in the same hand.` |
| 4 | **Позовите семью** | `Three herbarium sheets of the same design lying slightly overlapped on a desk, as if three people had each brought their own, with a single ribbon loosely tying them together.` |

Each: `[PREAMBLE] <subject line> Hairline copperplate engraving, fine hatching, restrained
sanguine accent on one element only, cream vellum ground, generous margins, flat-on view.`

**Deliver:** 2400×1800 PNG transparent each.

---

## 5. Display-mode thumbnails (set of 8)

Small engraved diagrams that illustrate each display mode in the mode switcher. These are
**diagrams, not illustrations** — they must be schematically accurate.

| Mode | Subject |
|---|---|
| Атлас | A small node-link family diagram, three generations |
| Веер | A half-circle fan divided into three concentric rings of segments |
| Песочные часы | An hourglass silhouette formed from two opposed triangular node arrays |
| Стратиграфия | Horizontal geological strata bands with vertical life bars crossing them |
| Потоки | A small Sankey diagram, three sources flowing into two destinations |
| Родство | Two marked points connected by a highlighted path through a small graph |
| Генограмма | The standard genogram square-and-circle notation for one nuclear family |
| География | A fragment of an engraved old map with three small location markers |

**Prompt pattern:** `[PREAMBLE] A small precise schematic diagram in the style of a figure from a
19th-century scientific treatise: <subject>. Drawn with hairline rules and simple geometric
forms, no labels or text, no decoration. Terracotta red used for a single emphasised element.
Isolated, centred, on plain cream vellum.`

**Deliver:** 1200×1200 PNG transparent each.

---

## 6. Sample data for the demo tree

The demo tree needs faces. Real family photographs cannot be used, and modern stock photography
would destroy the tone.

**Prompt:** `[PREAMBLE — with one modification: this image IS photographic] A studio portrait
photograph in the style of the 1890s, made on a glass plate negative: a <age> <person> facing
slightly off-camera, in period-appropriate clothing of a Russian provincial family, plain
painted backdrop, soft single-source light from the left, shallow depth of field, visible
silver-gelatin grain, slight edge vignetting, faded sepia-brown tone, small emulsion
imperfections and one fine scratch. Head and shoulders. Neutral expression, dignified.`

Produce roughly 12: four in the 1890s style, four in a 1920s–30s style, four in a 1950s–60s
style, varied in age and sex.

**Mandatory:** every sample portrait is stored with `synthetic: true` in its metadata, is
watermarked in the corner of the demo build, and lives only in `assets/demo/`. It must be
impossible for a synthetic face to end up in someone's real archive.

---

## 7. Drawn by us, not generated — the icon set

**Not an image-model task.** Roughly 80 icons on a 24 px grid, drawn as SVG, in an engraved
style: open hairline forms, no solid fills, the same five line weights as the rest of the system.

Every icon ships **with a visible text label** in the interface, per
[elder-ux-and-accessibility.md](../01-research/elder-ux-and-accessibility.md).

Groups: navigation (12), person and family actions (16), evidence and sources (14), media (10),
sync and contributions (12), view modes (8), system (8).

Tracked as a Phase 2 task with its own checklist in `packages/ui/icons/`.

---

## 8. Better than generating: public-domain plates

For ornament, background plates and reference material, genuine 19th-century engravings are
**free, authentic and legally clean**. They will beat generated imagery on texture and line
quality, and using them is faithful to the concept rather than imitating it.

| Source | What is there | Licence |
|---|---|---|
| **Biodiversity Heritage Library** — biodiversitylibrary.org | Hundreds of thousands of scanned botanical and zoological plates; a dedicated Flickr collection of extracted illustrations | Overwhelmingly public domain; check per item |
| **Internet Archive** | Full scanned atlases and herbals | Public domain for pre-1930 works |
| **Rijksmuseum Studio** | Very high-resolution engravings, downloadable | Public domain |
| **NYPL Digital Collections** | Maps, plates, ornament | Public domain subset marked |
| **Old Book Illustrations** | Curated, pre-extracted, transparent | Public domain |
| **David Rumsey Map Collection** | Historical maps — ideal for the geography mode ground | CC BY-NC-SA, check per item |

**Process:** select, verify the licence, record the provenance in `assets/source/ATTRIBUTION.md`,
then process to our palette (desaturate, re-tint to the ink token, clean the background). A
short script in `tools/assets/` does the re-tinting consistently.

**[judgement]** Use this route for corner ornaments, dividers and the geography ground. Use
generation for the app icon, empty states and onboarding, where we need a specific composition
that no historical plate happens to contain.

---

## 9. Delivery checklist

```
assets/
├── source/                    ← originals at full resolution, committed via Git LFS
│   ├── identity/
│   ├── ornament/
│   ├── illustration/
│   ├── modes/
│   └── ATTRIBUTION.md         ← provenance and licence for every public-domain item
├── demo/                      ← synthetic sample portraits, watermarked
└── generated/                 ← build output; git-ignored
```

| # | Asset | Count | Priority |
|---|---|---|---|
| A1 | App icon | 1 + 3 variants | **Phase 1** |
| A2 | Android adaptive layers | 2 | Phase 1 |
| A3 | Social preview | 1 | Phase 1 |
| 2.3 | Foxing overlay | 1 | Phase 3 |
| 2.4 | Deckle and plate mark | 1 | Phase 3 |
| 3.1 | Corner ornaments | 4 | Phase 2 |
| 3.2 | Dividers | 6 | Phase 2 |
| 3.3 | Seal | 2 | Phase 2 |
| 4.1–4.4 | Empty states | 4 | **Phase 1** |
| 4.5 | Onboarding plates | 4 | **Phase 1** |
| 5 | Mode thumbnails | 8 | Phase 2 |
| 6 | Sample portraits | 12 | Phase 1 |

Procedural (no generation needed): grain tiles, hatching, halftone.
Drawn by us: the 80-icon set.

**Start with the Phase 1 rows.** They unblock the first runnable build; everything else can
follow once the interface exists to put it in.
