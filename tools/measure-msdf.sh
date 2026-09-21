#!/usr/bin/env bash
# Measures what an MSDF glyph atlas costs, per typeface instance and per coverage tier.
#
# Spike S3. Text is the bottleneck in the canvas: at z2 and z3 every visible person carries a
# name, and the names in this product are long Russian ones. Multi-channel signed distance
# fields give crisp text at any zoom from one texture — but only for glyphs that are in the
# texture. The question is where to draw that line.
#
# Two things make the measurement non-obvious, and both are handled here rather than assumed:
#
#   1. Every *weight* needs its own atlas. msdf-atlas-gen 1.4's `-varfont` flag silently ignores
#      the axis settings in this build — two atlases baked at wght=200 and wght=900 come out
#      byte-identical — so the script instantiates static instances with fontTools first and
#      verifies they actually differ.
#   2. The JSON metrics are a substantial fraction of the payload, so they are reported
#      separately rather than folded into the atlas figure.
#
# Requires: msdf-atlas-gen (https://github.com/Chlumsky/msdf-atlas-gen/releases),
#           Python with fontTools, Node.
#
#   tools/measure-msdf.sh <path-to-msdf-atlas-gen> <font-dir> [out-dir]

set -euo pipefail

GEN="${1:?usage: measure-msdf.sh <msdf-atlas-gen> <font-dir> [out-dir]}"
FONTS="${2:?usage: measure-msdf.sh <msdf-atlas-gen> <font-dir> [out-dir]}"

cd "$(dirname "$0")/.."
OUT="${3:-target/msdf-size}"
mkdir -p "$OUT/instances"

# ── the instances the canvas actually draws with ───────────────────────────
# Format: source-file | axis settings (empty for a static font) | label
#
# The canvas needs far fewer than the full design system: it draws names and dates, not prose.
# Interface chrome is DOM text and uses the web fonts directly, costing no atlas at all.
INSTANCES=(
  "SourceSerif4-Var.ttf|wght=400,opsz=14|SourceSerif4-400"
  "SourceSerif4-Var.ttf|wght=600,opsz=14|SourceSerif4-600"
  "GolosText-Var.ttf|wght=400|GolosText-400"
  "GolosText-Var.ttf|wght=600|GolosText-600"
  "JetBrainsMono-Var.ttf|wght=400|JetBrainsMono-400"
  "Prata-Regular.ttf||Prata-400"
)

# ── coverage tiers ─────────────────────────────────────────────────────────
# A · what the interface needs on day one: ASCII, Russian, the German additions, and the
#     typographic punctuation Russian text requires — «ёлочки», em and en dashes, №.
# B · A plus Latin-1, Latin Extended-A, Greek and the full Cyrillic block: the rest of Europe,
#     and the Greek that turns up in scholarly citation.

cat > "$OUT/tier-a.txt" <<'CHARSET'
[0x20, 0x7E]
[0x0410, 0x044F]
0x0401, 0x0451
0xC4, 0xD6, 0xDC, 0xE4, 0xF6, 0xFC, 0xDF
0xAB, 0xBB, 0x2013, 0x2014, 0x2018, 0x2019, 0x201C, 0x201D, 0x201E
0x2116, 0x00A0, 0x00B7, 0x2026, 0x2212, 0x00D7
CHARSET

cat > "$OUT/tier-b.txt" <<'CHARSET'
[0x20, 0x7E]
[0x00A0, 0x00FF]
[0x0100, 0x017F]
[0x0370, 0x03FF]
[0x0400, 0x04FF]
[0x2010, 0x203A]
0x2116, 0x20AC, 0x20BD, 0x2212, 0x00D7
CHARSET

kib() { awk -v b="$1" 'BEGIN { printf "%.0f KiB", b / 1024 }'; }

# ── instantiate ────────────────────────────────────────────────────────────
echo "Instantiating static weights…"
for entry in "${INSTANCES[@]}"; do
  IFS='|' read -r src axes label <<< "$entry"
  target="$OUT/instances/$label.ttf"
  if [[ -z "$axes" ]]; then
    cp "$FONTS/$src" "$target"
  else
    python - "$FONTS/$src" "$target" "$axes" <<'PY'
import sys
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

src, dst, axes = sys.argv[1], sys.argv[2], sys.argv[3]
settings = {}
for pair in axes.split(','):
    tag, value = pair.split('=')
    settings[tag] = float(value)

font = TTFont(src)
instancer.instantiateVariableFont(font, settings, inplace=True)
font.save(dst)
PY
  fi
done

# Guard against the failure this script exists to avoid: if two weights of one family produce
# identical files, the axis was not applied and every per-weight number below is fiction.
dupes=$(md5sum "$OUT"/instances/*.ttf | awk '{print $1}' | sort | uniq -d | wc -l)
if [[ "$dupes" -gt 0 ]]; then
  echo "error: two instances are byte-identical — a variation axis was not applied." >&2
  exit 1
fi
echo "  ${#INSTANCES[@]} distinct instances."
echo

# ── bake and measure ───────────────────────────────────────────────────────
: > "$OUT/results.txt"
printf '%-20s %-5s %7s %11s %10s %10s\n' "instance" "tier" "glyphs" "atlas" "png" "json"
printf '%s\n' "---------------------------------------------------------------------"

for entry in "${INSTANCES[@]}"; do
  IFS='|' read -r _ _ label <<< "$entry"
  for tier in a b; do
    stem="$OUT/$label-$tier"
    if ! "$GEN" -font "$OUT/instances/$label.ttf" -type msdf -format png \
        -imageout "$stem.png" -json "$stem.json" -charset "$OUT/tier-$tier.txt" \
        -size 48 -pxrange 4 -pots > "$stem.log" 2>&1; then
      printf '%-20s %-5s %7s %11s %10s %10s\n' "$label" "$tier" "—" "FAILED" "—" "—"
      continue
    fi
    read -r glyphs dims <<< "$(node -e "
      const j = require('./$stem.json');
      process.stdout.write(j.glyphs.length + ' ' + j.atlas.width + 'x' + j.atlas.height);
    ")"
    png=$(stat -c %s "$stem.png")
    json=$(stat -c %s "$stem.json")
    printf '%-20s %-5s %7s %11s %10s %10s\n' \
      "$label" "$tier" "$glyphs" "$dims" "$(kib "$png")" "$(kib "$json")"
    echo "$label $tier $glyphs $dims $png $json" >> "$OUT/results.txt"
  done
done

echo
echo "Totals for the full canvas set:"
for tier in a b; do
  read -r png json <<< "$(awk -v t="$tier" '$2==t { p += $5; j += $6 } END { print p+0, j+0 }' "$OUT/results.txt")"
  printf '  tier %s: %s atlas + %s metrics = %s\n' \
    "$tier" "$(kib "$png")" "$(kib "$json")" "$(kib "$((png + json))")"
done

echo
echo "Budget: full Russian and German coverage under 4 MB."
echo "Artefacts in $OUT/"
