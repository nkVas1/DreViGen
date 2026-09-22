# Herbarium Vivum · plate 14

80 original SVG icons on a 24 px grid. Source geometry: `tools/assets/vectors.mjs`.
Every glyph uses `currentColor`, a 1.5 px stroke, and rounded line caps and joins.

The files live in `14-interface/`, with seven numbered categories. `manifest.json`
and `labels.ts` contain Russian visible labels; `sprite.svg` is the combined export.
The browser must receive the external sprite from the same origin.

```html
<button class="dv-icon-label" type="button">
  <svg aria-hidden="true" focusable="false" viewBox="0 0 24 24">
    <use href="/icons/14-interface/sprite.svg#dv-home"></use>
  </svg>
  <span>Домой</span>
</button>
```

The text is part of the control. A tooltip or an `aria-label` alone does not replace
the visible label. Use at least 44 × 44 px for interactive targets; 48 × 48 px for
primary actions. In high contrast, increase the stroke to 2 px.

Run `npm --prefix tools/assets run build` from the repository root to regenerate.
The SVGs have no third-party icon dependency and no remote image/font references.
