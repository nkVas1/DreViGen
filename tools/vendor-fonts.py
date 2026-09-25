#!/usr/bin/env python3
"""Vendors the chosen typefaces: downloads, subsets, and writes woff2 plus attribution.

The faces are chosen in `docs/03-design/typefaces.md` and are not re-litigated here. This
script turns that decision into files the application can load.

Fonts are **vendored rather than linked**. Three reasons, in order of weight: the desktop and
mobile builds have no network at first paint and a web font that arrives late is a page that
reflows under the reader; a third-party font host sees every reader of every family tree, which
is not a request a genealogy application should make on its users' behalf; and a pinned file
cannot change under us the way a hosted one can.

Subsetting is not optional either. A full variable face carries every script its designer
shipped, and we need Latin, Cyrillic and a short list of punctuation — see the tiers in
`ADR 0009`, which the MSDF atlas uses too.

Usage:  python tools/vendor-fonts.py [--out assets/fonts]
"""

from __future__ import annotations

import argparse
import io
import sys
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path

from fontTools import subset
from fontTools.ttLib import TTFont

GOOGLE_FONTS = "https://raw.githubusercontent.com/google/fonts/main"


@dataclass(frozen=True)
class Face:
    """A vendored typeface."""

    voice: str
    family: str
    file: str
    """Path within the google/fonts repository."""
    licence: str
    """Path to the licence file within the same repository."""
    designer: str
    note: str
    css_family: str = ""
    """The family name as CSS will refer to it."""
    weight_range: str = "400"
    """`font-weight` descriptor: a range for a variable face, a number for a static one."""
    display: str = "swap"
    """`font-display`. The marginalia face is optional, so it may arrive late or not at all."""
    preload: bool = True
    """Whether first paint waits for it."""


# Settled in docs/03-design/typefaces.md by measurement, not preference.
FACES = [
    Face("display", "Prata", "ofl/prata/Prata-Regular.ttf", "ofl/prata/OFL.txt",
         "Cyreal", "Didone with complete Cyrillic; single weight, which display type does not need",
         css_family="Prata", weight_range="400"),
    Face("text", "SourceSerif4", "ofl/sourceserif4/SourceSerif4%5Bopsz%2Cwght%5D.ttf",
         "ofl/sourceserif4/OFL.txt", "Frank Grießhammer, Adobe",
         "variable on optical size and weight; tabular figures and a slashed zero",
         css_family="Source Serif 4", weight_range="200 900"),
    Face("data", "GolosText", "ofl/golostext/GolosText%5Bwght%5D.ttf", "ofl/golostext/OFL.txt",
         "ParaType", "contemporary Russian grotesque; tabular figures",
         css_family="Golos Text", weight_range="400 900"),
    Face("hand", "Caveat", "ofl/caveat/Caveat%5Bwght%5D.ttf", "ofl/caveat/OFL.txt",
         "Pablo Impallari, Impallari Type", "marginalia only; Cyrillic confirmed by the audit",
         css_family="Caveat", weight_range="400 700", display="optional", preload=False),
    Face("mono", "JetBrainsMono", "ofl/jetbrainsmono/JetBrainsMono%5Bwght%5D.ttf",
         "ofl/jetbrainsmono/OFL.txt", "JetBrains", "archival references; slashed zero",
         css_family="JetBrains Mono", weight_range="100 800"),
]

# The same coverage the MSDF atlas pre-bakes (ADR 0009 tier A), so the two cannot drift.
UNICODES_TIER_A = (
    "U+0020-007E,"          # ASCII printable
    "U+00A0,"               # no-break space
    "U+00B7,U+00D7,"        # middle dot, multiplication
    "U+00C4,U+00D6,U+00DC," # ÄÖÜ
    "U+00E4,U+00F6,U+00FC," # äöü
    "U+00DF,"               # ß
    "U+0401,U+0451,"        # Ёё
    "U+0410-044F,"          # Russian
    "U+00AB,U+00BB,"        # «»
    "U+2013,U+2014,"        # – —
    "U+2018,U+2019,U+201C,U+201D,U+201E,"
    "U+2026,"               # …
    "U+2116,"               # №
    "U+2212"                # minus
)

# Every layout feature is retained rather than a curated list.
#
# The first version named the features we thought we needed, and it silently dropped `tnum`
# from Source Serif 4 — present in the original, absent from the subset — while keeping it in
# Golos Text. Whatever the cause, a subsetter that removes a feature the audit recorded as
# present is a subsetter whose output no longer matches the audit, and the size saved is a few
# kilobytes against a class of defect that shows up as misaligned figures months later.
LAYOUT_FEATURES = ["*"]


def fetch(url: str) -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": "DreViGen-font-vendor/0.1"})
    with urllib.request.urlopen(request, timeout=120) as response:
        return response.read()


def vendor(face: Face, out: Path) -> tuple[int, int, int]:
    """Downloads and subsets one face. Returns (original, subset ttf, woff2) byte sizes."""
    raw = fetch(f"{GOOGLE_FONTS}/{face.file}")
    original = len(raw)

    font = TTFont(io.BytesIO(raw))
    options = subset.Options()
    options.layout_features = LAYOUT_FEATURES
    options.name_IDs = ["*"]
    options.name_legacy = True
    options.notdef_outline = True
    options.recalc_bounds = True
    # Keep the variation tables: a variable face that loses fvar becomes a single weight, and
    # every extra static weight is another MSDF atlas.
    options.drop_tables = ["DSIG"]

    subsetter = subset.Subsetter(options=options)
    subsetter.populate(unicodes=subset.parse_unicodes(UNICODES_TIER_A))
    subsetter.subset(font)

    ttf_buffer = io.BytesIO()
    font.save(ttf_buffer)
    subset_size = ttf_buffer.tell()

    font.flavor = "woff2"
    woff2_path = out / f"{face.family}-subset.woff2"
    font.save(woff2_path)
    font.close()

    return original, subset_size, woff2_path.stat().st_size


def stylesheet(rows: list[tuple[Face, int]]) -> str:
    """Emits the @font-face rules and the family stacks, from the same data as the files."""
    lines = [
        "/* DreViGen vendored typefaces — GENERATED by tools/vendor-fonts.py. Do not edit.",
        " *",
        " * Chosen by measurement: docs/03-design/typefaces.md",
        " * Subset to the coverage the MSDF atlas pre-bakes: ADR 0009 tier A",
        " */",
        "",
    ]
    for face, size in rows:
        lines += [
            f"/* {face.css_family} — {face.voice}, {size / 1024:.0f} KiB. {face.note} */",
            "@font-face {",
            f"  font-family: '{face.css_family}';",
            # One src, one format. The old `woff2-variations` plus `woff2` pair was a
            # workaround for browsers that no longer exist, and it made some of them fetch the
            # same file twice.
            f"  src: url('./{face.family}-subset.woff2') format('woff2');",
            f"  font-weight: {face.weight_range};",
            "  font-style: normal;",
            f"  font-display: {face.display};",
            "}",
            "",
        ]

    lines += [
        "/* The stacks. Prata lacks … and №, so the text face follows it and the browser",
        " * substitutes those two characters without anyone noticing. */",
        ":root {",
        "  --dv-font-display: 'Prata', 'Source Serif 4', Georgia, 'Times New Roman', serif;",
        "  --dv-font-text:    'Source Serif 4', Georgia, serif;",
        "  --dv-font-data:    'Golos Text', system-ui, -apple-system, 'Segoe UI', sans-serif;",
        "  --dv-font-hand:    'Caveat', 'Segoe Script', cursive;",
        "  --dv-font-mono:    'JetBrains Mono', ui-monospace, Consolas, monospace;",
        "}",
        "",
        "/* Figures that sit in a column must align, and a year is a column of one. */",
        ".dv-tabular, [data-dv-figures='tabular'] {",
        "  font-variant-numeric: tabular-nums lining-nums;",
        "}",
        "",
    ]
    return "\n".join(lines)


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", default="assets/fonts")
    args = parser.parse_args()

    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)

    print(f"{'voice':<9} {'family':<16} {'original':>10} {'subset':>10} {'woff2':>10}  saving")
    print("─" * 70)

    rows = []
    total = 0
    for face in FACES:
        try:
            original, subset_size, woff2 = vendor(face, out)
        except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError) as error:
            print(f"{face.voice:<9} {face.family:<16} download failed: {error}")
            return 1
        total += woff2
        saving = 100.0 * (1.0 - woff2 / original)
        print(
            f"{face.voice:<9} {face.family:<16} {original / 1024:>9.0f}K "
            f"{subset_size / 1024:>9.0f}K {woff2 / 1024:>9.0f}K  {saving:>5.1f}%"
        )

        licence = fetch(f"{GOOGLE_FONTS}/{face.licence}").decode("utf-8", "replace")
        (out / f"{face.family}-OFL.txt").write_text(licence, encoding="utf-8", newline="\n")
        rows.append((face, woff2))

    print("─" * 70)
    print(f"{'':<9} {'total':<16} {'':>10} {'':>10} {total / 1024:>9.0f}K")

    attribution = ["# Vendored Typefaces",
                   "",
                   "**Generated by `tools/vendor-fonts.py`. Do not edit.**",
                   "",
                   "Chosen by measurement in [typefaces.md](../../docs/03-design/typefaces.md);",
                   "subset to the same coverage the MSDF atlas pre-bakes",
                   "([ADR 0009](../../docs/02-architecture/adr/0009-msdf-atlas-tiers.md) tier A).",
                   "",
                   "Every face is under the SIL Open Font License 1.1. The full licence text for each",
                   "sits beside it as `<Family>-OFL.txt`, as the OFL requires.",
                   "",
                   "| Voice | Family | Designer | Subset | Why this one |",
                   "|---|---|---|---|---|"]
    for face, size in rows:
        attribution.append(
            f"| {face.voice} | {face.family} | {face.designer} | {size / 1024:.0f} KiB | {face.note} |"
        )
    attribution += [
        "",
        "## Why vendored rather than linked",
        "",
        "The desktop and mobile builds have no network at first paint, and a web font that arrives",
        "late is a page that reflows under the reader. A third-party font host also sees every",
        "reader of every family tree, which is not a request a genealogy application should make on",
        "its users' behalf. And a pinned file cannot change under us the way a hosted one can.",
        "",
        "## Coverage",
        "",
        "Latin, Russian, the German additions, and the typographic punctuation Russian text",
        "requires. Anything outside that falls back to the platform's own fonts — which is correct,",
        "since a tree containing Greek or Hebrew sources is rare enough that everyone should not",
        "download those glyphs on every first paint.",
        "",
    ]
    (out / "ATTRIBUTION.md").write_text("\n".join(attribution), encoding="utf-8", newline="\n")
    (out / "fonts.css").write_text(stylesheet(rows), encoding="utf-8", newline="\n")
    print(f"\nWrote {out}/fonts.css, {out}/ATTRIBUTION.md and {len(rows)} licence files.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
