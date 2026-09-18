#!/usr/bin/env bash
# Reproducible kz vs wc benchmark suite.
#   ./bench/run.sh            # run all
# Requires: hyperfine, python3, GNU coreutils wc.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA="${KZ_BENCH_DATA:-$ROOT/bench/data}"
KZ="${KZ_BIN:-$ROOT/target/release/kz}"
W=3; R=10

command -v hyperfine >/dev/null || { echo "need hyperfine: cargo install hyperfine"; exit 1; }
[ -x "$KZ" ] || { echo "need release build: cargo build --release"; exit 1; }
[ -f "$DATA/big-ascii.txt" ] && [ -f "$DATA/big-utf8.txt" ] || python3 "$ROOT/bench/gen_data.py" "$DATA"

echo "host: $(uname -srm)"
echo "cpu:  $(lscpu | sed -n 's/^Model name: *//p' | head -1) ($(nproc) threads)"
echo "wc:   $(wc --version | head -1)"
echo "kz:   $("$KZ" --version)"
echo

# warm the page cache so we measure CPU, not disk
cat "$DATA"/big-ascii.txt "$DATA"/big-utf8.txt "$DATA"/corpus/*.txt >/dev/null

bench() { # label, args, target
  echo "## $1"
  hyperfine -w $W -r $R --style basic \
    -n "wc (UTF-8)"  "wc $2 $3" \
    -n "wc (LC_ALL=C)" "LC_ALL=C wc $2 $3" \
    -n "kz"          "$KZ $2 $3"
  echo
}

bench "1GB ASCII — longest line (-L)"  "-L" "$DATA/big-ascii.txt"
bench "500MB UTF-8 — longest line (-L)" "-L" "$DATA/big-utf8.txt"
bench "1GB file — words (-w)"        "-w" "$DATA/big-ascii.txt"
bench "1GB file — default (l/w/c)"   ""   "$DATA/big-ascii.txt"
bench "1GB file — lines (-l)"        "-l" "$DATA/big-ascii.txt"
bench "500 files (218MB) — default"  ""   "$DATA/corpus/*.txt"
bench "4KB file — startup floor"     ""   "$DATA/small.txt"
