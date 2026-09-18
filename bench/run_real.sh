#!/usr/bin/env bash
# Reproduce the published table in BENCHMARKS.md against the real corpora.
# Run ./bench/prep_real.sh first.
#
#   ./bench/run_real.sh
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA="${KZ_REAL_DATA:-$ROOT/bench/real}"
KZ="${KZ_BIN:-$ROOT/target/release/kz}"

command -v hyperfine >/dev/null || { echo "need hyperfine: cargo install hyperfine"; exit 1; }
[ -x "$KZ" ] || { echo "need release build: cargo build --release"; exit 1; }
[ -s "$DATA/prose.txt" ] || { echo "no corpora: run ./bench/prep_real.sh first"; exit 1; }

cd "$DATA"
echo "cpu:  $(lscpu | sed -n 's/^Model name: *//p' | head -1) ($(nproc) threads)"
echo "wc:   $(wc --version | head -1)"
echo "kz:   $("$KZ" --version)"
echo

# Counts must agree before any timing is reported: a faster wrong answer is not
# a result. This is the check that caught both of the bugs these numbers exist for.
echo "verifying agreement with wc..."
fail=0
for f in prose.txt cjk.txt; do
  for a in -l -w -c -m -L; do
    x=$(wc $a "$f" | awk '{print $1}'); y=$("$KZ" $a "$f" | awk '{print $1}')
    [ "$x" = "$y" ] || { echo "  DIVERGE $f $a: wc=$x kz=$y"; fail=1; }
  done
done
[ "$fail" -eq 0 ] || { echo "counts disagree with wc — not benchmarking"; exit 1; }
echo "  ok: all modes match wc on prose.txt and cjk.txt"
echo

# Warm the page cache so we measure CPU, not the disk.
cat prose.txt prose-1gb.txt cjk.txt books/*.txt >/dev/null 2>&1

H="hyperfine -w 3 -r 10 --style basic"
row() { echo "## $1"; shift; $H "$@" 2>&1 | grep -E 'Time \(mean|times faster'; echo; }

# LC_ALL=C is shown where wc does less work in the C locale. On -m it answers a
# different question there (it degrades to a byte count), so the UTF-8 row is the
# like-for-like comparison — see the "-m row" section in BENCHMARKS.md.
row "-m, 100MB real prose"     -n wc "wc -m prose.txt"     -n "wc (C)" "LC_ALL=C wc -m prose.txt"     -n kz "$KZ -m prose.txt"
row "-L, 1GB real prose"       -n wc "wc -L prose-1gb.txt" -n "wc (C)" "LC_ALL=C wc -L prose-1gb.txt" -n kz "$KZ -L prose-1gb.txt"
row "-w, 1GB real prose"       -n wc "wc -w prose-1gb.txt" -n "wc (C)" "LC_ALL=C wc -w prose-1gb.txt" -n kz "$KZ -w prose-1gb.txt"
row "default, 1GB real prose"  -n wc "wc prose-1gb.txt"    -n "wc (C)" "LC_ALL=C wc prose-1gb.txt"    -n kz "$KZ prose-1gb.txt"
row "-w, 100MB real prose"     -n wc "wc -w prose.txt"     -n "wc (C)" "LC_ALL=C wc -w prose.txt"     -n kz "$KZ -w prose.txt"
row "141 real files, 100MB"    -n wc "sh -c 'wc books/*.txt'" -n kz "sh -c '$KZ books/*.txt'"
row "-m, 6.7MB CJK"            -n wc "wc -m cjk.txt"       -n "wc (C)" "LC_ALL=C wc -m cjk.txt"       -n kz "$KZ -m cjk.txt"
row "stdin -w, 100MB"          -n wc "sh -c 'cat prose.txt | wc -w'" -n kz "sh -c 'cat prose.txt | $KZ -w'"
row "--files0-from, .rs tree"  -n wc "sh -c 'xargs -0 wc -l < code.files0'" -n kz "$KZ -l --files0-from code.files0"
row "-l, 1GB real prose"       -n wc "wc -l prose-1gb.txt" -n kz "$KZ -l prose-1gb.txt"
row "recursive tree walk"      -n wc "sh -c 'find code -name \"*.rs\" -print0 | xargs -0 wc -l'" -n kz "$KZ -l -r code --exclude '*.md'"
row "stdin -l, 100MB"          -n wc "sh -c 'cat prose.txt | wc -l'" -n kz "sh -c 'cat prose.txt | $KZ -l'"
hyperfine -w 50 -r 500 -N --style basic -n wc "wc small.txt" -n kz "$KZ small.txt" 2>&1 | grep -E 'Time \(mean|times faster'
