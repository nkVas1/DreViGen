# Phase 0 — the foundation screen

*Captured 2026-09-22, at 1280 px, 100 % text scale, in all three themes.*

Not a phase exit capture — Phase 0 is still open on the device verifications. This is the
before-and-after the archive's README asks for on a deliberate visual redirection: the
Herbarium Vivum collection landing in the product.

## What changed

**The paper is real.** The grain was `feTurbulence`, which costs nothing and renders
differently in every engine — Chrome, WebKit and the Tauri webview each produced their own
paper, which on a product whose entire surface is one sheet is a difference visible between two
of the owner's own devices. It is now a seeded, seamless 512 px tile, identical everywhere,
48 KB in the theme that is showing.

**The specimen is an engraving.** The seedling was a shape sketched in SVG by hand. It is now
plate 10 of the collection, drawn in ink on a transparent ground.

**The rule is botanical.** A straight `<hr>` became the oak divider, drawn as a mask rather
than shown as a picture, so it takes the theme's rule colour instead of being frozen in the grey
it was engraved in.

**The icon set is in.** Eighty glyphs, and the component that renders them will not render one
without its label.

## What the three captures show

**Light.** The intended reading. The engraving floats on the vellum with no container; the
ornament is a whisper; the page is one sheet with a plate mark.

**Dark.** The engraving is ink, and ink on a lamplit ground disappears — so it is laid on a
sheet of vellum. That is not a workaround for the theme, it is the metaphor the interface is
built on: a plate resting on a lit desk. Worth looking at again when there is a second
illustration, because one sheet is a plate and three would be clutter.

## What the contrast capture proves

Everything decorative is gone: no grain, no ornament, the rule is a straight line, the glyph
strokes thicken from 1.5 to 2. The engraving stays, because it is the subject rather than
decoration, and it keeps no container because the ground is already white.

This is the behaviour the art direction promises in §4 and §10, verified rather than asserted.

## Open, and visible in these captures

- The engraving was generated against an earlier ink, so it reads a little cooler than the
  gated `ink`. Raster plates are deliberately outside the artwork palette gate — an engraving
  is ink on paper and its shades are the point — but the next raster generation should use the
  gated hexes. `tools/assets/production-plan.mjs` says so at the top.
- The ornament at `--dv-rule` is very quiet in the light theme. Deliberate for now. Revisit
  once there is content around it to judge it against.
