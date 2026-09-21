# Visual Asset Production Brief

> **The working package is [`asset-brief.html`](./asset-brief.html)** — fourteen plates, each with
> an engraved sketch, a specimen label, a copy-ready prompt and a delivery spec, plus live colour
> swatches and type specimens. Open it in a browser; GitHub shows HTML as source.
>
> This file holds the parts that belong in text: the policy, the manifest, and the sourcing rules.
> **Prompts and sketches live only in the HTML**, so there is exactly one copy of each to drift.

Companion to [art-direction.md](./art-direction.md) — read that first; both assume it.

---

## What must not be generated

| Category | Why not | Instead |
|---|---|---|
| **Icons and symbols** (~80) | Must be pixel-aligned, stroke-consistent and legible at 16 px. A model cannot hold a 1 px stroke grid across 80 files. | Drawn as SVG on a 24 px grid, in `packages/ui/icons/` |
| **Tileable grain and hatching** (9) | A model cannot guarantee seamless tiling, and one visible seam runs across the whole interface. | Procedural, `tools/textures/` |
| **Anything containing text** | Cyrillic in generated images is unreliable. | All text is live type, composited by the build |

## Manifest

| Plate | Asset | Files | Method | Phase |
|---|---|---|---|---|
| I | App icon | 4 | generate | **1** |
| II | Adaptive layers, social card | 3 | generate | **1** |
| III | Paper grain | 3 | procedural | **1** |
| IV | Hatching and halftone | 6 | procedural | **1** |
| V | Foxing | 1 | generate | 3 |
| VI | Deckle and plate mark | 1 | generate | 3 |
| VII | Corner vignettes | 4 | public domain / generate | 2 |
| VIII | Dividers | 6 | public domain / generate | 2 |
| IX | The seal | 2 | generate | 2 |
| X | Empty states | 4 | generate | **1** |
| XI | Onboarding plates | 4 | generate | **1** |
| XII | Display-mode thumbnails | 8 | draw or generate | 2 |
| XIII | Sample portraits | 12 | generate (photographic) | **1** |
| XIV | Icon set | ~80 | drawn as SVG | 2 |

**Phase 1 unblocks the first runnable build: plates I, II, X, XI, XIII (27 files) plus the two
procedural sets.** Everything else waits until there is an interface to put it in.

## Delivery

```text
assets/
├── source/            working masters — optimised, not full resolution
│   ├── identity/  ornament/  illustration/  modes/
│   └── ATTRIBUTION.md
├── demo/              synthetic sample portraits, watermarked
└── generated/         build output, git-ignored
```

- **Generate at 4×** the largest use size, then downsample. Fine hatching does not survive otherwise.
- **Naming:** `<category>-<name>-<variant>@<scale>.png`, e.g. `ornament-corner-tl@4x.png`.
- **Every file carries a sibling `<name>.prompt.txt`** with the exact prompt, model and seed.
  Without it the set cannot be extended consistently and will drift.
- **Never hand-resize.** `tools/assets/` produces the derivatives.

### Resolution policy

**Full-resolution originals stay out of the repository.** A few dozen 4000 px plates is hundreds
of megabytes; clone weight is permanent, and Git LFS free quota (1 GB storage, 1 GB bandwidth per
month) would not survive it on a public repo. Commit the largest size the product uses at 2×, as
AVIF q85 or PNG where transparency detail demands it; keep the originals locally and publish a
`design-masters-<date>.zip` to GitHub Releases once per phase. Regeneration is cheap when the
prompt and seed are recorded. Full policy in [`assets/README.md`](../../assets/README.md).

## Public-domain plates — better than generating

For ornament, background plates and the cartographic ground, genuine 19th-century engravings are
free, legally clean, and better than generated imagery on line quality. Using them is faithful to
the concept rather than an imitation of it.

| Source | What is there | Licence |
|---|---|---|
| [Biodiversity Heritage Library](https://www.biodiversitylibrary.org/) | Hundreds of thousands of botanical and zoological plates | Mostly public domain — verify per item |
| [Rijksmuseum Studio](https://www.rijksmuseum.nl/en/rijksstudio) | Very high-resolution engravings | Public domain |
| [NYPL Digital Collections](https://digitalcollections.nypl.org/) | Maps, plates, ornament | Marked public-domain subset |
| [Old Book Illustrations](https://www.oldbookillustrations.com/) | Curated, pre-extracted, transparent | Public domain |
| [David Rumsey Map Collection](https://www.davidrumsey.com/) | Historical maps for the geography mode | CC BY-NC-SA — verify per item |
| [Internet Archive](https://archive.org/) | Whole scanned atlases and herbals | Public domain pre-1930 |

**Process:** select → verify the licence → record provenance in `assets/source/ATTRIBUTION.md` →
re-tint to the `ink` token with the script in `tools/assets/`. An unattributed asset is a legal
problem that cannot be unwound later.

## Two rules that are not stylistic

**Synthetic portraits.** Every sample portrait is stored with `synthetic: true` in its metadata,
watermarked in the demo build, and confined to `assets/demo/`. It must be impossible for a
synthetic face to reach a real family archive.

**Every icon ships with a visible text label** — not a tooltip, a label. Older users are
measurably more often confused by unlabelled iconography, so the label is a functional part of the
control. CI rejects an interactive element without one. See
[elder-ux-and-accessibility.md](../01-research/elder-ux-and-accessibility.md).

## Open finding

**Archivo, named in the art direction as the data face, has no Cyrillic** — Latin, Latin-ext and
Vietnamese only. The HTML package uses **Golos Text** (Paratype) instead: a contemporary Russian
grotesque with full Cyrillic and tabular figures. Resolve in the Phase 1 type audit and update
[art-direction.md](./art-direction.md) §5.
