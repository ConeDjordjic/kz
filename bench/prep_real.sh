#!/usr/bin/env bash
# Fetch and build the real corpora behind the published numbers in BENCHMARKS.md.
# Needs network on first run; everything is content-addressed by Gutenberg id and
# by Cargo.lock, so repeated runs and other machines get the same corpora.
#
#   ./bench/prep_real.sh [dest]        default dest: bench/real
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${1:-$ROOT/bench/real}"
GUT="https://www.gutenberg.org/cache/epub"

# 141 Project Gutenberg texts: mixed 19th/20th century English literature.
BOOK_IDS="11 12 43 55 58 62 64 74 76 84 98 105 113 120 121 135 141 145 158 160 161 174 203 205 209 215 219 236 244 271 289 345 375 408 514 521 543 600 730 766 768 829 844 863 902 932 967 996 1023 1064 1080 1112 1155 1184 1232 1250 1259 1322 1342 1399 1400 1404 1497 1513 1661 1727 1952 1998 2000 2148 2200 2350 2500 2542 2554 2591 2600 2701 2814 3207 3296 3600 4300 4363 4517 5200 5230 5740 6130 6593 7370 8800 9296 10007 11030 12242 13415 14975 16328 16643 17989 18857 19033 20203 21700 23042 24022 25344 25717 26740 27525 27827 28054 28885 29021 30254 30601 31284 32325 33283 33511 34901 35899 36034 37106 38769 39407 40367 41445 42324 43936 45631 46423 47629 48320 49304 50133 51014 52091 53416 54100 "
# 4 CJK texts, ~96% non-ASCII, for the multibyte case.
CJK_IDS="23950 23962 24264 25328"

mkdir -p "$DEST/books" "$DEST/cjk"

fetch() { # id, dir
  local id="$1" dir="$2"
  [ -s "$dir/$id.txt" ] && return 0
  curl -sS --max-time 60 -o "$dir/$id.txt" "$GUT/$id/pg$id.txt" || { rm -f "$dir/$id.txt"; return 1; }
  # a Gutenberg error page is far smaller than any real text
  [ "$(stat -c%s "$dir/$id.txt")" -ge 5120 ] || { rm -f "$dir/$id.txt"; return 1; }
}

echo "fetching 141 English texts (skipping any already present)..."
missing=0
for id in $BOOK_IDS; do fetch "$id" "$DEST/books" || { echo "  warn: id $id unavailable"; missing=$((missing+1)); }; done
echo "fetching 4 CJK texts..."
for id in $CJK_IDS; do fetch "$id" "$DEST/cjk" || { echo "  warn: id $id unavailable"; missing=$((missing+1)); }; done
[ "$missing" -gt 0 ] && echo "NOTE: $missing text(s) missing — corpus differs from the published one, so numbers will not match exactly."

echo "building corpora..."
cat "$DEST"/books/*.txt > "$DEST/prose.txt"
cat "$DEST"/cjk/23950.txt "$DEST"/cjk/23962.txt "$DEST"/cjk/24264.txt "$DEST"/cjk/25328.txt > "$DEST/cjk.txt"
# 1GB file: the same real text repeated 10x. Counting is a linear scan, so the
# repetition does not flatter the result, but these rows measure throughput at
# scale rather than corpus diversity.
: > "$DEST/prose-1gb.txt"
for _ in $(seq 1 10); do cat "$DEST/prose.txt" >> "$DEST/prose-1gb.txt"; done
head -c 4096 "$DEST/prose.txt" > "$DEST/small.txt"

# Code corpus: the crate sources kz itself locks, so it is identical on any
# machine with the same Cargo.lock. (A raw ~/.cargo/registry/src tree is not
# reproducible — it holds whatever that machine happened to build.)
echo "vendoring locked crate sources..."
if [ ! -d "$DEST/code" ]; then
  tmp="$(mktemp -d)"; trap 'rm -rf "$tmp"' EXIT
  cp "$ROOT/Cargo.toml" "$ROOT/Cargo.lock" "$tmp/"
  mkdir -p "$tmp/src" && echo 'fn main() {}' > "$tmp/src/main.rs"
  ( cd "$tmp" && cargo vendor "$DEST/code" >/dev/null )
fi
# paths are relative to $DEST, because run_real.sh runs from inside it
( cd "$DEST" && find code -name '*.rs' -print0 > code.files0 )

echo
echo "corpora ready in $DEST:"
printf "  %-16s %12s bytes  %9s lines  %10s words\n" prose.txt "$(stat -c%s "$DEST/prose.txt")" "$(wc -l < "$DEST/prose.txt")" "$(wc -w < "$DEST/prose.txt")"
printf "  %-16s %12s bytes\n" prose-1gb.txt "$(stat -c%s "$DEST/prose-1gb.txt")"
printf "  %-16s %12s bytes\n" cjk.txt "$(stat -c%s "$DEST/cjk.txt")"
printf "  %-16s %12s .rs files\n" code/ "$(tr -dc '\\0' < "$DEST/code.files0" | wc -c)"
