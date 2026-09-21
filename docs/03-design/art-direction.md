# Art Direction — *Herbarium Vivum* / «Живой гербарий»

*Accepted 2026-09-21. This document is binding on every visual decision in the project.*

---

## 1. The idea in one sentence

**A family archive presented as a living scientific atlas** — each person a specimen on a plate,
each family a page, the whole tree a territory you navigate like a map, with the researcher's
evidence living in the margins where a naturalist would write it.

## 2. Why this and not something else

Four requirements had to be satisfied at once, and they normally pull apart:

| Requirement | Why the botanical-atlas synthesis satisfies it |
|---|---|
| **Legible for an 80-year-old** | The evidence says older users prefer high contrast, **skeuomorphic** cues and labelled icons. A plate, a card, a pressed specimen and a handwritten label are objects he already understands. Nothing here is an abstract glyph he has to learn. |
| **Not template, not AI-slop** | The 2026 anti-sameness movement converges on **tactile materiality** and craft; texture "reads as premium because it looks slow and deliberate to produce". A scientific-illustration system is about as far from the default dark-SaaS-gradient as a product can get. |
| **Scales to 50 000 people** | The atlas metaphor brings **semantic zoom** with it for free — and honestly. Maps change their symbols with scale; so does this. |
| **Serves a researcher** | Marginalia *is* the apparatus. Field notes, specimen labels, collection numbers, an index and a gazetteer are not decoration layered onto a research tool — they are literally what a research tool is made of. |

The historical-strata mode and the Sankey flows are not stylistic add-ons either: a scientific
atlas contains stratigraphic sections and flow diagrams. They belong to the same document.

## 3. The five layers

Every surface in DreViGen is assembled from these, in this order. Nothing else exists.

```
  ┌───────────────────────────────────────────────────────────────┐
  │  5. МАРГИНАЛИИ    field notes, citations, queries, corrections │  ← evidence
  ├───────────────────────────────────────────────────────────────┤
  │  4. АППАРАТ       minimap, legend, scale, landmarks, index     │  ← navigation
  ├───────────────────────────────────────────────────────────────┤
  │  3. ОБРАЗЕЦ       the person: dot → sprig → card → plate       │  ← content
  ├───────────────────────────────────────────────────────────────┤
  │  2. ОТТИСК        engraved line, hatching, halftone, rules     │  ← structure
  ├───────────────────────────────────────────────────────────────┤
  │  1. ОСНОВА        vellum: paper tone, grain, deckle, fold      │  ← ground
  └───────────────────────────────────────────────────────────────┘
```

**1 · Основа (the ground).** A warm vellum surface with fine grain and an almost imperceptible
laid-paper structure. One tiled noise texture with a CSS blend mode — never a per-frame shader.
Deckle edges and a plate-mark border appear only on print-destined artefacts, never in working
chrome.

**2 · Оттиск (the impression).** Hairline engraved rules, cross-hatching for density, halftone
for photographic zones. Line weights come from a fixed set of five (0.5 / 0.75 / 1 / 1.5 / 2 px
at 1× density) so the whole product looks cut by the same tool.

**3 · Образец (the specimen).** The person, rendered at whichever of five symbol grades the
current zoom calls for. Details in §6.

**4 · Аппарат (the apparatus).** The cartographic furniture: minimap, legend, generation scale,
landmarks, compass-home. Always present, always labelled in words.

**5 · Маргиналии (the marginalia).** Everything evidential — citation marks, confidence,
research queries, someone else's correction — lives in the margin in a hand, visually distinct
from the printed body. This is the Workshop layer: present but quiet in the Hall, foregrounded
in the Workshop.

## 4. Colour

Authored in **OKLCH**, gamut-mapped to sRGB, validated with **APCA** and WCAG 2. Eleven-stop
ramps per family (50–950); the anchors below are the 500-ish identity stops.

### Light — «Дневной свет» (default)

| Token | OKLCH | ≈ hex | Role |
|---|---|---|---|
| `vellum` | `oklch(96.5% 0.012 85)` | `#FBF7EE` | The page. Every surface starts here. |
| `vellum-deep` | `oklch(93.2% 0.016 82)` | `#F1E9DA` | Recessed panels, plate wells |
| `ink` | `oklch(22% 0.022 250)` | `#1E232B` | Primary text and engraved line. Iron-gall bias — a blue-black, never pure black. |
| `ink-soft` | `oklch(43% 0.020 250)` | `#4E555F` | Secondary text, hairlines |
| `sanguine` | `oklch(58% 0.132 42)` | `#B4552C` | **The single accent.** Focus, the current person, the primary action. Red chalk. |
| `sage` | `oklch(55% 0.055 142)` | `#6B8163` | Verified, sourced, confirmed |
| `sepia` | `oklch(52% 0.072 66)` | `#8A6A44` | Inferred, approximate, uncertain |
| `indigo` | `oklch(41% 0.092 266)` | `#3C4E8C` | The historical strata layer |
| `verdigris` | `oklch(56% 0.066 186)` | `#3E8A86` | Places and geography |
| `madder` | `oklch(51% 0.145 25)` | `#A63D34` | Conflict, contradiction, open question |

### Dark — «Лампа» (night reading room)

Not an inversion. A desk under a lamp: the paper goes to deep warm brown-black, the ink becomes
the warm light. Chroma is reduced and lightness rebalanced per token — never computed by flipping
the L channel, because that is exactly the case where WCAG 2 contrast maths misleads and APCA is
required.

| Token | OKLCH | Role |
|---|---|---|
| `vellum` | `oklch(19% 0.012 62)` | Lamplit desk |
| `vellum-deep` | `oklch(15% 0.010 62)` | Recesses |
| `ink` | `oklch(91% 0.014 85)` | Warm light text |
| `sanguine` | `oklch(66% 0.125 44)` | Accent, lifted for dark ground |

### Rules

1. **One accent.** `sanguine` marks the focal person and the primary action. If two things on
   screen are sanguine, one of them is wrong.
2. **Colour is never the only signal.** Confidence carries a shape and a label as well as a hue.
   Every state survives greyscale and every common colour-vision deficiency.
3. **Branch tinting is generated, not chosen.** Family branches receive hues at even OKLCH
   spacing from a seeded rotation, with L and C fixed — so a 12-branch tree is always
   distinguishable and never garish.
4. **High-contrast mode is a first-class theme**, not a filter: texture off, hairlines to 1.5 px
   minimum, APCA Lc ≥ 90 for body text.

## 5. Typography

Three voices, each with a job, all open-licensed with genuine Cyrillic.

*Settled by measurement on 2026-09-21 — see [typefaces.md](./typefaces.md) for the full audit
and the arguments. Two faces named in the first draft of this document turned out to have no
Cyrillic at all.*

| Voice | Face | Use |
|---|---|---|
| **Заголовок** — the plate | **Prata** (OFL, Cyreal) | Plate titles, person names at z3–z4, section heads. A true Didone with complete Cyrillic. High contrast, engraved authority. Display sizes only — never below 24 px. Missing `…` and `№`, which fall back to the text face. |
| **Текст** — the page | **Source Serif 4 Variable** (OFL) | Body, notes, biographies, research prose. Variable on optical size and weight, with tabular figures and a slashed zero. |
| **Данные** — the label | **Golos Text Variable** (OFL, ParaType) | Dates, counts, tables, UI controls, specimen labels. Variable weight, **tabular figures** — a column of years must align. Chosen over Inter, which is complete but is the generic default the brief exists to avoid; the slashed zero it gives up is needed only in archival shelf marks, which are set in the mono voice. |
| **Рука** — the margin | **Caveat Variable** (OFL) | Marginalia only: research queries, personal notes, corrections in flight. Never for data, never below 16 px, never load-bearing. Cyrillic coverage confirmed by the audit. |
| **Шифр** — the reference | **JetBrains Mono** (OFL) | Archival references only (`ГААО ф.29 оп.1 д.204 л.17об`), identifiers, GEDCOM tags. **Slashed zero**, which is why shelf marks belong here. |

**Not used, and recorded so they are not proposed again:** *Theano Didot* is not distributable
through the normal font pipeline; *Archivo* and *Bodoni Moda* contain **no Cyrillic whatsoever**;
*Playfair Display* is complete and is one of the most-used serifs on the web, which costs more
than the two characters Prata lacks.

### Scale

Fluid, with a 1.25 ratio, floored for legibility. **Body default 19 px, absolute floor 17 px.**
The document scale control (100 / 125 / 150 / 200 %) multiplies the whole scale and **reflows**;
it does not zoom a bitmap.

```
  caption   15 → 17    label    17 → 19    body      19 → 21
  lead      24 → 27    title    30 → 38    plate     47 → 61
```

### Rules

- Measure 60–75 characters for prose. Never full-bleed text.
- **Never Prata below 24 px** — Didone hairlines disappear and it becomes an accessibility
  failure rather than a style.
- Numerals in any column or comparison are tabular, always.
- Russian text is set with proper typography: «ёлочки», non-breaking spaces before dashes,
  hanging punctuation where the renderer allows.

## 6. The specimen — five symbol grades

This is the semantic-zoom system and the single most distinctive thing in the product. **Scale
changes the symbol, not its size.** Positions are scale-invariant; only symbology and detail
change, and transitions cross-fade without any reflow.

```
z0  ТЕРРИТОРИЯ            z1  ВЕТВЬ                 z2  СЕМЬЯ
────────────────          ─────────────────         ───────────────────
 · · ·  · ·  · ·           ⌇ ВАСИЛЬЕВЫ               ┌─────────────┐
· · ·· ··· ·· ·            ⌇                         │ ИВАН        │
 ·· ···●··· ·· ·           ⌇── ⌇ ── ⌇                │ 1871–1943   │
· ·· ··· ··· ·             ⌇   ⌇    ⌇                └──────┬──────┘
 · ·  · ·  ·               ⌇ ПЕТРОВЫ                        │
                                                     ┌──────┴──────┐
seed-dots, branch hue     sprigs; surname clusters   pressed-specimen card
generation bands          life-span bars visible     name · years · marks


z3  ОБРАЗЕЦ                              z4  ДОСЬЕ
──────────────────────────               ─────────────────────────────────
┌──────────────────────────┐             ┌──────────────────────────────────┐
│ ╔══════╗                 │             │ ╔══════╗  ИВАН ПЕТРОВИЧ          │
│ ║ ◕    ║  ИВАН           │             │ ║ ◕    ║  ВАСИЛЬЕВ               │
│ ║      ║  Васильев       │             │ ╚══════╝  1871 – 1943            │
│ ╚══════╝                 │             │ ─────────────────────────────────│
│ 1871 – 1943              │             │ Рождение  17.IV.1871  Подгорное  │
│ с. Подгорное             │             │ Венчание  09.II.1894  Подгорное  │
│ ─────────────────────    │             │ Смерть    03.XI.1943  Архангельск│
│ ✦✦✦  4 источника         │             │ ─────────────────────────────────│
└──────────────────────────┘             │  ✎ «в переписи 1897 — 1873 г.»   │
   full plate: portrait,                 │    → открытый вопрос             │
   name, dates, place,                   └──────────────────────────────────┘
   source marks                             the plate opens in place —
                                            the canvas is never left
```

### The marks on a specimen

Small, consistent, always labelled on first encounter through the legend:

| Mark | Meaning | Rendering |
|---|---|---|
| `✦✦✦` | Source count and quality | Engraved asterisks, `sage` when cited, outline when not |
| `◔` | Living person | Quarter-filled disc, privacy-aware |
| `⟠` | Contested fact present | `madder` lozenge — the honest signal that sources disagree |
| `✎` | Open research question | `sanguine` hand-drawn nib |
| `⌘` | Pinned by the user | Sanguine pin head |
| `◈` | Direct ancestor of "me" | Filled diamond in the branch hue |

**Every one of these appears in the always-available legend with a text explanation.** This is
the direct implementation of the research finding that older users are confused by unlabelled
iconography — the legend is not a help screen, it is permanent furniture.

## 7. The apparatus

```
┌────────────────────────────────────────────────────────────────────────┐
│  ДРЕВО ВАСИЛЬЕВЫХ                      [ Зал ] [ Мастерская ]   ⌂ Домой│  ← always
├──────────────┬─────────────────────────────────────────────────────────┤
│              │                                                          │
│  ⌖ ОБЗОР     │                    · · · ·                               │
│  ┌────────┐  │                  · ▓▓▓▓ · ·                              │
│  │  ░░▓░  │  │                · · ▓▓●▓▓ · ·           ← canvas          │
│  │  ░▓█▓░ │  │                  · ▓▓▓▓ · ·                              │
│  │  ░░▓░  │  │                    · · · ·                               │
│  └────────┘  │                                                          │
│              │                                                          │
│  ▣ ЛЕГЕНДА   │                                                          │
│  ✦ источники │                                                          │
│  ⟠ разночтен.│                                                          │
│  ◔ живой     │                                                          │
│              │                                                          │
│  ⌂ МЕТКИ     │                                                          │
│  · я         │                                                          │
│  · дальний   │                                                          │
│    предок    │                                                          │
│  · последнее │                                                          │
│    изменение │                                                          │
├──────────────┴─────────────────────────────────────────────────────────┤
│  1 : 8 поколений    ├───────┼───────┤    1790 ────────◆──────── 2026    │
└────────────────────────────────────────────────────────────────────────┘
```

Non-negotiable elements:

- **⌂ Домой** — one control, one position, one label, forever. Returns to the user's own record
  at a readable zoom. It is the answer to "I am lost", and it never moves.
- **⌖ Обзор** (minimap) — always visible, with the viewport rectangle. The only way to understand
  where you are in a 50 000-person territory.
- **▣ Легенда** — every symbol currently on screen, explained in words. Collapsible, never
  removable.
- **Generation scale** — reads in generations, not pixels, because "1 : 8 поколений" is
  meaningful and "37 %" is not.
- **Time ruler** — the tree's temporal extent with the current focus marked. Doubles as the entry
  point to the strata mode.

## 8. Display modes

The canvas is one surface with several readings. Switching is a **View Transition**, ~420 ms,
emphasised easing — slow enough to follow the morph, never slow enough to annoy.

| Mode | Form | Reads best for |
|---|---|---|
| **Атлас** (default) | Semantic-zoom node-link plate | Everything; the working surface |
| **Веер** | Fan chart, generations as rings | One person's ancestry; the best print artefact |
| **Песочные часы** | Ancestors above, descendants below | A focal person in both directions |
| **Стратиграфия** | Time vertical, lives as bars, history stratum beneath | What each ancestor lived through |
| **Потоки** | Sankey | Migration between places; descendant volume by generation |
| **Родство** | Highlighted path between two people | "How am I related to her?" |
| **Генограмма** | Clinical/behavioural annotation | Medical and relationship patterns |
| **География** | Events on a map with period boundaries | Where the family was, and when |

### Стратиграфия in detail

```
        │ 1850 ─────────────────────────────────────────────────────
        │      ╟─── ИВАН ВАСИЛЬЕВ ────────────────────╢
        │ 1875 ─────────────────────────────────────────────────────
        │            ╟─── АННА ─────────────────────────────╢
        │            ╟─ ПЁТР ──╢ †1914
        │ 1900 ─────────────────────────────────────────────────────
        │  ▓▓▓▓▓▓ Русско-японская   ▓▓▓▓▓▓▓▓▓▓ Первая мировая
        │ 1925 ─────────────────────────────────────────────────────
        │  ▓▓▓▓ коллективизация  ▓▓ голод 1932–33
        │ 1950 ─────────────────────────────────────────────────────
        │      ▓▓▓▓▓▓▓▓▓▓ Великая Отечественная
        └────────────────────────────────────────────────────────────
           ▓ historical stratum (indigo, regionally scoped, shipped as data)
```

A life is a **bar**, not a point. Beneath the personal bars runs a stratum of historical events,
scoped to the relevant region. This is simultaneously an emotional feature ("he was 19 in 1914")
and a research instrument: knowing what someone lived through tells you which record sets ought
to exist for them.

### Потоки (Sankey) — used precisely

Sankey is correct for exactly two things and is banned elsewhere:

1. **Migration** — place of birth → place of death, flow width by number of people, across
   generations. Instantly shows a family leaving a village for a city.
2. **Descendant volume** — how many descendants each child line produced, generation by
   generation. Shows which branches flourished and which ended.

## 9. Motion

The physical model is **paper**: things are lifted, laid down, pressed and turned. Nothing
bounces. Nothing springs playfully. The archive is calm.

| Gesture | Motion | Parameters |
|---|---|---|
| Pan / zoom | Spring-driven, velocity-carrying, interruptible | stiffness 420, damping 30 |
| Specimen card opens | Lifts (shadow deepens), expands in place | 280 ms, ease-out; spring on scale, damping 28 |
| Card closes | Settles back, shadow flattens | 220 ms, ease-in |
| Zoom band crossing | Cross-fade of symbology, **no reflow** | 180 ms, linear on opacity |
| Display-mode change | View Transition, morphing shared elements | 420 ms, emphasised |
| Panel enters | Slide + fade from its own edge | 240 ms, ease-out |
| Validation error | Two-cycle horizontal shake, 3 px | 160 ms — suppressed under reduced motion |
| Save committed | Ink-settle: the value darkens to final tone | 200 ms, ease-out |
| Contribution merged | A seal presses onto the page, once | 500 ms, emphasised; the one ceremonial motion in the product |
| Staggered lists | 30 ms per item, maximum 8 items | ease-out |

### Rules

1. **Every animation answers a question.** Where did this come from; where did it go; what
   changed; is it still working. An animation that answers nothing is deleted.
2. **The same interaction always animates identically.** Consistency is part of the language.
3. **Interruption is always honoured.** Spring physics, so reversing a gesture reverses from
   current velocity rather than jumping.
4. **`prefers-reduced-motion` removes movement, not feedback.** Transitions become cross-fades
   at 120 ms; nothing translates; every state change still announces itself. An in-app override
   exists so nobody has to find an OS setting.
5. **Motion never delays input.** Anything beyond 200 ms is interruptible and the underlying
   state has already committed.

## 10. Photography and imagery

Family photographs are the emotional core and must never be subordinated to the styling.

- **Presentation:** a photograph sits in a **plate well** — a recessed area with a hairline
  rule and a subtle paper shadow. It is never tinted, never duotoned, never overlaid with
  gradients. The frame is ours; the image is theirs.
- **Aspect is preserved.** No forced crops. Vertical portraits get vertical wells.
- **Original is sacred.** Enhancement, colourisation and restoration produce derivatives with
  recorded provenance. A viewer can always reach the untouched scan in one action.
- **Generated content is labelled** in the UI, in exports, in print and in file metadata.
- **Placeholders are not silhouettes.** A person without a photograph gets a **pressed-specimen
  card** — a botanical plate mark in their branch hue with their initial set in Theano Didot.
  Dignified, personal, and never the grey head-and-shoulders icon that reads as "missing".

## 11. Ethics of representation

Stated here because it is an art-direction decision, not only a policy one.

We can technically animate a photograph of someone's dead grandmother. MyHeritage built a viral
business on it. **We decline to make it a headline feature**, for a specific reason: a family
archive's value rests entirely on the trust that what it contains is *true*. A moving, smiling
ancestor who never moved or smiled that way overwrites a real memory with a synthetic one, and
it does so most powerfully on exactly the people we are building for. The cost is borne by the
person least able to detect it.

If it ever ships it is opt-in per photograph, indelibly watermarked, excluded from archival
export, and never shown by default. The same reasoning governs colourisation, which *is*
offered, because an obviously interpretive colour pass reads as an interpretation in a way that
a moving face does not.

## 12. The ownability test

Remove the logo, the name and all the text from any screenshot. If it is not recognisably
DreViGen from the vellum ground, the engraved line, the sanguine accent, the specimen grades and
the cartographic apparatus, the design has failed and gets redone.

This test is run at the end of every phase, on real screens, and the result is archived in
[`design-archive/`](../../design-archive/).
