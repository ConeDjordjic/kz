#!/usr/bin/env bash
# Assert kz and wc produce identical counts. A benchmark is meaningless if they disagree.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA="${KZ_BENCH_DATA:-$ROOT/bench/data}"
KZ="${KZ_BIN:-$ROOT/target/release/kz}"
fail=0

# wc -L and wc -m are locale-dependent: under LC_ALL=C they degrade to bytes.
# Comparing in a UTF-8 locale is the whole point of the big-utf8.txt case.
case "$(locale charmap 2>/dev/null)" in
  UTF-8|utf8) ;;
  *) echo "FAIL locale is not UTF-8 ($(locale charmap 2>/dev/null)); -L/-m comparisons would be meaningless"; exit 1;;
esac

for file in big-ascii.txt big-utf8.txt; do
  path="$DATA/$file"
  [ -f "$path" ] || { echo "FAIL missing $path — run bench/gen_data.py"; fail=1; continue; }
  echo "== $file"
  for args in "-l" "-w" "-c" "-m" "-L" ""; do
    a=$(wc $args "$path" | tr -s ' ' | sed 's/^ //')
    b=$("$KZ" $args "$path" | tr -s ' ' | sed 's/^ //')
    if [ "$a" = "$b" ]; then echo "ok   ${args:-default}: $a"
    else echo "FAIL ${args:-default}: wc='$a' kz='$b'"; fail=1; fi
  done
done

# Canary: big-utf8.txt is only useful if its widest line is not pure ASCII.
# If bytes and columns ever agree on the maximum, this corpus has silently
# stopped being able to catch a bytes-instead-of-columns -L.
if [ -f "$DATA/big-utf8.txt" ]; then
  bytes_max=$(LC_ALL=C awk '{ if (length($0) > m) m = length($0) } END { print m+0 }' "$DATA/big-utf8.txt")
  cols_max=$(wc -L < "$DATA/big-utf8.txt")
  if [ "$bytes_max" != "$cols_max" ]; then
    echo "ok   canary: widest line is $cols_max columns in $bytes_max bytes"
  else
    echo "FAIL canary: byte-max == column-max ($cols_max); big-utf8.txt cannot catch a width bug"
    fail=1
  fi
fi

exit $fail
