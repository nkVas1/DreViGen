# Herbarium Vivum · asset provenance

Produced for DreViGen from `docs/03-design/asset-brief.html` and the project's art
direction, September 2026.

| Material | Origin | Provenance |
|---|---|---|
| Numbered application mark and adaptive layers | DreViGen's existing `drevigen-mark` Rust generator | Geometry and resolved palette from `assets/identity/`; originals unchanged |
| Engraved emblem, paper, botanical ornaments, empty states, onboarding | Built-in OpenAI `image_gen` | Exact prompts and edit prompts in sibling `.prompt.txt`; original/output hashes in `.provenance.json` |
| Synthetic demo portraits | Built-in OpenAI `image_gen` | Entirely fictional; confined to `assets/demo/`, visible marking + embedded XMP + JSON |
| Mode diagrams and 80 interface icons | Original SVG geometry authored for DreViGen | `tools/assets/vectors.mjs`; no external icon pack |
| Grain, hatching, halftone | Seeded/analytic procedural artwork | `tools/textures/generate-grain.mjs` |
| Fonts used by the catalogue | Existing vendored project fonts | See `assets/fonts/ATTRIBUTION.md` and each OFL file |

No external historical plate, stock photograph, or public-domain scan was included.
The package therefore does not depend on an unverified public-domain attribution.
The exact backend model revision and random seed were not exposed by the built-in
image tool. They are explicitly recorded as unavailable, not guessed.

Original programmatic code and SVG artwork follow the repository's AGPL-3.0-only
licence. Generated raster assets are identified as generated; this document makes
no claim of exclusive authorship in machine-generated pixels.
