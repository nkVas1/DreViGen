# Assets

Production specifications, prompts and sketches live in
[`docs/03-design/asset-brief.md`](../docs/03-design/asset-brief.md). This file covers only
**where files go and in what form**.

## Layout

```
assets/
├── source/            Working masters, committed — optimised, not full-resolution
│   ├── identity/      App icon, adaptive layers, social preview
│   ├── ornament/      Corners, dividers, the seal
│   ├── illustration/  Empty states, onboarding plates
│   ├── modes/         Display-mode thumbnails
│   └── ATTRIBUTION.md Provenance and licence for every public-domain item
├── demo/              Synthetic sample portraits for the demo tree — watermarked
└── generated/         Build output. Git-ignored.
```

## Resolution policy — read before committing anything

**Full-resolution masters do not go in this repository.**

Generated assets arrive at 2000–4000 px. A few dozen of those is hundreds of megabytes, which
makes every clone expensive forever, and Git LFS free quota (1 GB storage, 1 GB bandwidth per
month) would be exhausted quickly on a public repository.

So:

| Where | What | Format |
|---|---|---|
| `assets/source/` | The largest size the product actually uses, ×2 | AVIF q85, or PNG where transparency detail demands it |
| **Outside the repository** | The original full-resolution generations | Kept by the author, with the `.prompt.txt` sidecars |
| GitHub Releases | A `design-masters-<date>.zip` published once per phase | Whatever the originals are |

Regeneration is cheap when the prompt and seed are recorded. Repository weight is permanent.
That is the trade, and it points one way.

## Every file needs its prompt

A generated asset is committed with a sibling `<name>.prompt.txt` containing the exact prompt,
the model and the seed. Without it the asset cannot be extended consistently in two years, and
the set will drift.

## Public-domain material

Anything sourced from Biodiversity Heritage Library, Rijksmuseum, NYPL, Internet Archive or
David Rumsey gets an entry in `source/ATTRIBUTION.md`: what it is, where it came from, the
licence, the date checked, and a direct link. No exceptions — an unattributed asset is a legal
problem we cannot unwind later.

## Synthetic portraits

`assets/demo/` holds generated period-style portraits for the demo tree. Every one is stored
with `synthetic: true` in its metadata and is watermarked in the demo build. It must be
impossible for a synthetic face to end up in a real family archive.
