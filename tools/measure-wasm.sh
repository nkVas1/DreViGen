#!/usr/bin/env bash
# Measures what the Rust core costs in bytes once compiled to WebAssembly.
#
# Spike S2. The roadmap budget is 2.5 MB gzipped before first paint. This script builds the
# spike crate at three feature levels, runs the full shipping pipeline on each — cargo →
# wasm-bindgen → wasm-opt -Oz → gzip — and reports the result against the budget.
#
# Feature levels are measured separately so the cost is attributed rather than reported as one
# number: knowing that Loro is most of it is what decides whether an eager/lazy split is needed.
#
# Usage:  tools/measure-wasm.sh [path-to-wasm-opt]
#
# wasm-opt comes from Binaryen (https://github.com/WebAssembly/binaryen/releases). If it is not
# on PATH and no path is given, the script reports unoptimised sizes and says so, rather than
# silently measuring the wrong thing.

set -euo pipefail

cd "$(dirname "$0")/.."

CRATE=s2-wasm-size
TARGET=wasm32-unknown-unknown
PROFILE=wasm-release
BUDGET_BYTES=$((2500 * 1024))

WASM_OPT="${1:-$(command -v wasm-opt || true)}"

# cargo install puts binaries in $CARGO_HOME/bin, which is not always on PATH under Git Bash.
WASM_BINDGEN="$(command -v wasm-bindgen || true)"
if [[ -z "$WASM_BINDGEN" ]]; then
  cargo_bin="${CARGO_HOME:-$HOME/.cargo}/bin"
  for candidate in "$cargo_bin/wasm-bindgen" "$cargo_bin/wasm-bindgen.exe"; do
    if [[ -x "$candidate" ]]; then
      WASM_BINDGEN="$candidate"
      break
    fi
  done
fi
if [[ -z "$WASM_BINDGEN" ]]; then
  echo "error: wasm-bindgen not found. Install it with:" >&2
  echo "  cargo install wasm-bindgen-cli --version 0.2.128 --locked" >&2
  exit 1
fi

OUT="target/wasm-size"
rm -rf "$OUT"
mkdir -p "$OUT"

if [[ -z "$WASM_OPT" ]]; then
  echo "note: wasm-opt not found — reporting unoptimised sizes, which overstate the real cost."
  echo
fi

printf '%-16s %>12s %12s %12s %12s\n' "" "" "" "" "" 2>/dev/null || true
printf '%-16s %12s %12s %12s %12s\n' "features" "cargo" "bindgen" "wasm-opt" "gzipped"
printf '%s\n' "------------------------------------------------------------------------"

measure() {
  local label="$1" features="$2" flags=()
  if [[ "$features" == "-" ]]; then
    flags=(--no-default-features)
  else
    flags=(--no-default-features --features "$features")
  fi

  cargo build -p "$CRATE" --target "$TARGET" --profile "$PROFILE" "${flags[@]}" -q

  local raw="target/$TARGET/$PROFILE/${CRATE//-/_}.wasm"
  local dir="$OUT/$label"
  mkdir -p "$dir"

  # wasm-bindgen strips its own custom sections and emits the JS glue the browser loads.
  "$WASM_BINDGEN" "$raw" --out-dir "$dir" --target web --no-typescript
  local bound="$dir/${CRATE//-/_}_bg.wasm"

  local final="$bound"
  if [[ -n "$WASM_OPT" ]]; then
    "$WASM_OPT" -Oz --enable-bulk-memory --enable-nontrapping-float-to-int \
      "$bound" -o "$dir/optimised.wasm"
    final="$dir/optimised.wasm"
  fi

  gzip -9 -c "$final" > "$dir/final.wasm.gz"

  local s_raw s_bound s_opt s_gz
  s_raw=$(stat -c %s "$raw")
  s_bound=$(stat -c %s "$bound")
  s_opt=$(stat -c %s "$final")
  s_gz=$(stat -c %s "$dir/final.wasm.gz")

  printf '%-16s %12s %12s %12s %12s\n' \
    "$label" "$(kib "$s_raw")" "$(kib "$s_bound")" "$(kib "$s_opt")" "$(kib "$s_gz")"

  echo "$label $s_gz" >> "$OUT/gzipped.txt"
}

kib() { awk -v b="$1" 'BEGIN { printf "%.0f KiB", b / 1024 }'; }

: > "$OUT/gzipped.txt"
measure "baseline"   "-"
measure "crdt"       "crdt"
measure "crdt+demo"  "crdt,demo"

echo
echo "JS glue emitted alongside each module:"
for d in "$OUT"/*/; do
  js=$(find "$d" -maxdepth 1 -name '*.js' | head -1)
  [[ -n "$js" ]] && printf '  %-16s %s\n' "$(basename "$d")" "$(kib "$(stat -c %s "$js")")"
done

echo
full=$(awk '/^crdt\+demo /{print $2}' "$OUT/gzipped.txt")
printf 'Budget: %s gzipped before first paint.\n' "$(kib "$BUDGET_BYTES")"
if [[ -n "${full:-}" && "$full" -lt "$BUDGET_BYTES" ]]; then
  printf 'Result: %s — PASS, %s of headroom.\n' "$(kib "$full")" "$(kib $((BUDGET_BYTES - full)))"
else
  printf 'Result: %s — FAIL. An eager/lazy split is required.\n' "$(kib "${full:-0}")"
fi

echo
echo "Artefacts in $OUT/"
