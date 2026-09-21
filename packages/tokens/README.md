# @drevigen/tokens

**Generated. Do not edit `dist/`.**

The palette lives in [`crates/drevigen-tokens/src/palette.rs`](../../crates/drevigen-tokens/src/palette.rs)
as OKLCH values. Everything here is derived from it, so a colour cannot be changed in one place
and go stale in another.

```sh
pnpm --filter @drevigen/tokens check      # audit contrast; exits non-zero on failure
pnpm --filter @drevigen/tokens generate   # audit, then rewrite dist/
```

## What is in here

| File | Contents |
|---|---|
| `dist/tokens.css` | Custom properties for all three themes, plus the eleven-stop ramps |
| `dist/tokens.ts` | The same colours as hex, for the canvas and anything that computes with them |
| `dist/contrast-report.txt` | Every pairing, measured — the record of what was checked |

## The gate

Forty-five pairings across three themes are checked on every build, with **APCA as the guide
and WCAG 2 as the conformance report**. A palette that cannot be read does not reach a branch.

This is not ceremony. Writing the first version of the contract failed twelve pairings, and
each failure was a real one — the accent was too light to be a button fill, the rules were
invisible, and the dark theme was guesswork. Two structural changes came out of fixing them:

- **The accent needed splitting.** `sanguine` is the accent *as ink*; `sanguine-fill` is the
  accent *as a surface*, dark enough in the light theme and light enough in the dark theme that
  a label on it can be read. One token could not do both.
- **One pairing was wrong rather than failing.** `ink on well` described a caption sitting on
  the recess a photograph occupies — which the product does not render; captions sit below the
  plate, on vellum. The real requirement for that surface is that the hairline framing the
  image is perceivable, so that is what the contract now checks.

## Why the generator is in Rust

The same [`drevigen-color`](../../crates/drevigen-color) crate runs in the canvas, which
generates family branch tints at runtime. Two implementations of OKLCH would drift, and the
drift would show up as a colour that passed the gate and is unreadable on screen.
