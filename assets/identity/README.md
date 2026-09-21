# The mark

Everything in this directory is **generated**. Do not edit it by hand; edit
`crates/drevigen-mark/src/lib.rs` and run:

```sh
cargo run -p drevigen-mark --features render -- assets/identity
```

The mark is a seed in cross-section whose growth rings are generations: three hand-irregular
rings, sixteen radial rays, and one filled dot of `sanguine` at the centre — the only colour in
it, per `docs/03-design/art-direction.md` §4.

| File | For |
|---|---|
| `mark.svg` / `mark-1024.png` | The source every other size is cut from |
| `mark-dark.*` | The lamplit tone, for dark UI surfaces |
| `mark-mono.*` | One colour: Windows taskbar, macOS template icons |
| `adaptive-foreground.*` | Android's foreground layer, inside the 66 % safe zone |
| `adaptive-background.*` | Android's background layer: the vellum ground, no detail |
| `adaptive-monochrome.*` | Android 13 themed icons: one shape on nothing, tinted by the system |

Colours are read from `drevigen-color`, so the mark cannot drift away from the palette the
contrast gate checks.

`tauri-icon.json` tells `cargo tauri icon` which layer is which. Expand the platform icon set
with:

```sh
cargo tauri icon -o apps/shell/src-tauri/icons assets/identity/tauri-icon.json
```

Regenerate it whenever this directory changes.

The asset brief assigns a higher-fidelity engraved mark (plate A1,
`docs/03-design/asset-brief.html`) to a generative model. When that arrives it replaces these
files. Until then the product has a real mark rather than a placeholder.
