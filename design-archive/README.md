# Design Archive

Curated visual snapshots at milestone points. **Not** a dumping ground for working screenshots.

## What belongs here

- **Phase exit captures.** At the end of every phase, the core screens in both themes, at
  100 % and 200 % text scale.
- **The ownability check.** The same screens with all branding, names and text removed. If the
  result is not recognisably DreViGen, the design has failed —
  see [art-direction.md](../docs/03-design/art-direction.md), final section. The failing
  captures are archived too; they are the evidence that the check has teeth.
- **Before and after** for any deliberate visual redirection, with a note on what changed and
  why.

## What does not belong here

Throwaway working shots. Those go in `.design-scratch/`, which is git-ignored.

## Naming

```
<phase>-<yyyy-mm-dd>-<screen>-<variant>.avif

p1-2026-12-04-atlas-light.avif
p1-2026-12-04-atlas-dark.avif
p1-2026-12-04-atlas-ownability.avif      ← branding stripped
p1-2026-12-04-dossier-light-200pct.avif
```

Each phase directory carries a `NOTES.md`: what changed since the previous phase, what the
ownability check concluded, and what the persona session surfaced.

## Format

AVIF, quality 80, at the capture resolution. Screenshots compress well and this archive will
be consulted for years — keep it light enough that cloning the repository stays cheap.
