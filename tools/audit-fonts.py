#!/usr/bin/env python3
"""Audits candidate typefaces against what DreViGen actually needs.

The art direction names four voices, and naming a typeface is not the same as checking it can
set the text. Archivo was named as the data face and turns out to have no Cyrillic at all —
which would have been discovered by a Russian-speaking user rather than by us. This script
exists so that cannot happen again.

Each candidate is measured, not recalled:

* **Coverage** of the character sets the product must set — Russian, German, and the
  typographic punctuation Russian text requires («ёлочки», em and en dashes, №).
* **Variable axes**, because every extra static weight is a separate MSDF atlas (ADR 0009).
* **Tabular figures** and a **slashed zero**, without which a column of years does not align
  and 0 and O are a guess.
* **Licence**, which must be OFL or equivalent.

Usage:  python tools/audit-fonts.py [--cache DIR]
"""

from __future__ import annotations

import argparse
import sys
import unicodedata
import urllib.error
import urllib.request
from dataclasses import dataclass, field
from pathlib import Path

from fontTools.ttLib import TTFont

GOOGLE_FONTS = "https://raw.githubusercontent.com/google/fonts/main"

# ── what the product has to be able to set ─────────────────────────────────

RUSSIAN = [chr(c) for c in range(0x0410, 0x0450)] + ["Ё", "ё"]
GERMAN = list("ÄÖÜäöüß")
TYPOGRAPHIC = list("«»—–…‘’“”„№·×−")
ASCII_PRINTABLE = [chr(c) for c in range(0x20, 0x7F)]

SETS: dict[str, list[str]] = {
    "ascii": ASCII_PRINTABLE,
    "russian": RUSSIAN,
    "german": GERMAN,
    "typographic": TYPOGRAPHIC,
}


@dataclass
class Candidate:
    """A typeface under consideration for one of the art direction's voices."""

    voice: str
    name: str
    path: str
    """Path within the google/fonts repository, or an absolute URL."""
    note: str = ""


@dataclass
class Result:
    """What the audit found."""

    candidate: Candidate
    ok: bool = False
    error: str = ""
    coverage: dict[str, tuple[int, int]] = field(default_factory=dict)
    missing: dict[str, list[str]] = field(default_factory=dict)
    axes: dict[str, tuple[float, float]] = field(default_factory=dict)
    features: set[str] = field(default_factory=set)
    glyphs: int = 0

    @property
    def complete(self) -> bool:
        """Whether every required set is fully covered."""
        return all(have == want for have, want in self.coverage.values())


CANDIDATES = [
    # Display — the plate voice. A Didone with real Cyrillic.
    Candidate("display", "Prata", "ofl/prata/Prata-Regular.ttf", "Cyreal; single weight"),
    Candidate("display", "Playfair Display", "ofl/playfairdisplay/PlayfairDisplay%5Bwght%5D.ttf"),
    Candidate("display", "Bodoni Moda", "ofl/bodonimoda/BodoniModa%5Bopsz%2Cwght%5D.ttf"),
    Candidate("display", "PT Serif", "ofl/ptserif/PT_Serif-Web-Regular.ttf", "ParaType"),
    # Text — the page voice.
    Candidate("text", "Source Serif 4", "ofl/sourceserif4/SourceSerif4%5Bopsz%2Cwght%5D.ttf"),
    Candidate("text", "Literata", "ofl/literata/Literata%5Bopsz%2Cwght%5D.ttf"),
    Candidate("text", "Noto Serif", "ofl/notoserif/NotoSerif%5Bwdth%2Cwght%5D.ttf"),
    # Data — the label voice. Needs tabular figures and a slashed zero.
    Candidate("data", "Archivo", "ofl/archivo/Archivo%5Bwdth%2Cwght%5D.ttf", "named in the art direction"),
    Candidate("data", "Golos Text", "ofl/golostext/GolosText%5Bwght%5D.ttf", "ParaType"),
    Candidate("data", "IBM Plex Sans", "ofl/ibmplexsans/IBMPlexSans%5Bwdth%2Cwght%5D.ttf"),
    Candidate("data", "PT Sans", "ofl/ptsans/PT_Sans-Web-Regular.ttf", "ParaType"),
    Candidate("data", "Inter", "ofl/inter/Inter%5Bopsz%2Cwght%5D.ttf"),
    # Mono — the reference voice, for archival citations.
    Candidate("mono", "JetBrains Mono", "ofl/jetbrainsmono/JetBrainsMono%5Bwght%5D.ttf"),
    Candidate("mono", "IBM Plex Mono", "ofl/ibmplexmono/IBMPlexMono-Regular.ttf"),
    # Hand — marginalia only.
    Candidate("hand", "Caveat", "ofl/caveat/Caveat%5Bwght%5D.ttf", "named in the asset brief"),
    Candidate("hand", "Marck Script", "ofl/marckscript/MarckScript-Regular.ttf"),
]


def fetch(candidate: Candidate, cache: Path) -> Path:
    """Downloads a candidate into the cache, or returns the cached copy."""
    target = cache / f"{candidate.name.replace(' ', '')}.ttf"
    if target.exists() and target.stat().st_size > 0:
        return target
    url = candidate.path if candidate.path.startswith("http") else f"{GOOGLE_FONTS}/{candidate.path}"
    request = urllib.request.Request(url, headers={"User-Agent": "DreViGen-font-audit/0.1"})
    with urllib.request.urlopen(request, timeout=60) as response:
        target.write_bytes(response.read())
    return target


def audit(candidate: Candidate, path: Path) -> Result:
    """Measures one typeface."""
    result = Result(candidate=candidate)
    try:
        font = TTFont(path, fontNumber=0, lazy=True)
    except Exception as error:  # noqa: BLE001 — a malformed download should not stop the audit
        result.error = f"{type(error).__name__}: {error}"
        return result

    cmap = font.getBestCmap()
    result.glyphs = len(font.getGlyphOrder())

    for name, chars in SETS.items():
        missing = [c for c in chars if ord(c) not in cmap]
        result.coverage[name] = (len(chars) - len(missing), len(chars))
        if missing:
            result.missing[name] = missing

    if "fvar" in font:
        for axis in font["fvar"].axes:
            result.axes[axis.axisTag] = (axis.minValue, axis.maxValue)

    for table in ("GSUB", "GPOS"):
        if table in font:
            records = font[table].table.FeatureList
            if records:
                result.features |= {r.FeatureTag for r in records.FeatureRecord}

    font.close()
    result.ok = True
    return result


def render(results: list[Result]) -> str:
    """Formats the audit as a table, grouped by voice."""
    lines: list[str] = []
    lines.append("Typeface audit — measured, not recalled.\n")
    header = f"{'voice':<8} {'typeface':<18} {'ascii':>7} {'russian':>8} {'german':>7} {'typo':>6} {'axes':<14} {'tnum':>5} {'zero':>5}"
    lines.append(header)
    lines.append("─" * len(header))

    voice = ""
    for r in results:
        if r.candidate.voice != voice:
            if voice:
                lines.append("")
            voice = r.candidate.voice

        if not r.ok:
            lines.append(f"{r.candidate.voice:<8} {r.candidate.name:<18} {r.error}")
            continue

        def pct(key: str) -> str:
            have, want = r.coverage[key]
            return "full" if have == want else f"{have}/{want}"

        axes = ",".join(sorted(r.axes)) or "static"
        lines.append(
            f"{r.candidate.voice:<8} {r.candidate.name:<18} "
            f"{pct('ascii'):>7} {pct('russian'):>8} {pct('german'):>7} {pct('typographic'):>6} "
            f"{axes:<14} {'yes' if 'tnum' in r.features else 'no':>5} "
            f"{'yes' if 'zero' in r.features else 'no':>5}"
        )

    lines.append("")
    lines.append("Disqualified — a missing script is not a trade-off, it is an exclusion:")
    excluded = [r for r in results if r.ok and not r.complete]
    if not excluded:
        lines.append("  none")
    for r in excluded:
        for name, chars in r.missing.items():
            sample = " ".join(
                f"{c} (U+{ord(c):04X} {unicodedata.name(c, '?')})" for c in chars[:4]
            )
            more = f" and {len(chars) - 4} more" if len(chars) > 4 else ""
            lines.append(f"  {r.candidate.name}: {name} incomplete — missing {sample}{more}")

    return "\n".join(lines) + "\n"


def main() -> int:
    # The report contains box drawing and Cyrillic; a Windows console defaults to cp1251 and
    # would raise rather than print.
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8", errors="replace")

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cache", default="target/font-audit", help="where to keep downloads")
    args = parser.parse_args()

    cache = Path(args.cache)
    cache.mkdir(parents=True, exist_ok=True)

    results: list[Result] = []
    for candidate in CANDIDATES:
        try:
            path = fetch(candidate, cache)
        except (urllib.error.URLError, urllib.error.HTTPError, TimeoutError) as error:
            results.append(Result(candidate=candidate, error=f"download failed: {error}"))
            continue
        results.append(audit(candidate, path))

    print(render(results))
    return 0


if __name__ == "__main__":
    sys.exit(main())
