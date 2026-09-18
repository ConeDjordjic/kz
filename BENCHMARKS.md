# Benchmarks

`kz` vs GNU `wc`, measured with [hyperfine](https://github.com/sharkdp/hyperfine)
(3 warmup runs, 10 measured runs, page cache warmed before each benchmark).

To reproduce the table below:

```bash
./bench/prep_real.sh     # fetch and build the corpora, about 1.2GB
./bench/run_real.sh      # check counts against wc, then time all 13 rows
```

`run_real.sh` verifies that `kz` and `wc` agree on `prose.txt` and `cjk.txt`
across all five counts, and exits without benchmarking if they do not. A
benchmark is meaningless if the two tools are not computing the same thing, and
that check is what catches it.

`./bench/run.sh` and `./bench/verify.sh` are the offline pair. They use
generated corpora from `bench/gen_data.py`, need no download, and are useful as
a quick check, but they will not reproduce the figures below.

## Corpora

These numbers are measured on real text. An earlier version of this file used
generated pure-ASCII corpora, and that hid two bugs: a `-L` that counted bytes
instead of display columns (invisible on ASCII, where the two are equal), and a
`-w` speedup that held only on ASCII and reversed on real prose. Both are
described under [Corrections](#corrections).

| Corpus | Size | What it is |
|---|---|---|
| `books/` | 100MB | 141 Project Gutenberg plain-text UTF-8 files, 19th and 20th century English literature |
| `prose.txt` | 104,329,273 B | `cat books/*.txt`. 2,011,399 lines, 17,650,663 words, 2.15% non-ASCII (typographic quotes, dashes, accented names) |
| `prose-1gb.txt` | 1,043,292,730 B | `prose.txt` repeated 10 times |
| `cjk.txt` | 7,076,058 B | 4 CJK Gutenberg texts, 96.3% non-ASCII |
| `code/` | 68,142,537 B | 2,972 `.rs` files, vendored from kz's own `Cargo.lock` |
| `small.txt` | 4KB | For the startup floor |

Books come from `https://www.gutenberg.org/cache/epub/<id>/pg<id>.txt`. The CJK
set is IDs 23950, 23962, 24264 and 25328. The code corpus is produced by
`cargo vendor` from this repository's own `Cargo.lock`, so it is byte-identical
on any machine with the same lockfile.

Two things worth stating plainly:

- The 1GB corpus is real text repeated, not 1GB of distinct books. Those rows
  measure throughput at scale, not corpus variety. Counting is a linear scan
  with no caching effects, so the repetition does not inflate the result, but
  it is repetition and a reader should be told.
- `prose.txt` at 2.15% non-ASCII is the corpus that matters. That is what real
  English text looks like, and it is what exposed the encoding cliff that a
  pure-ASCII corpus could never trigger. The generator in `bench/gen_data.py`
  is kept so the correctness check runs without a download, but the published
  numbers use the real corpora.

## Environment

| | |
|---|---|
| CPU | AMD Ryzen 7 7800X3D, 8 cores / 16 threads |
| OS | Linux 7.2.6-zen2 x86_64 |
| `wc` | GNU coreutils 9.11 |
| `kz` | 0.1.0 (release, `lto = "fat"`, `codegen-units = 1`) |

## Results

`kz` wins 11 of 13 workloads. Both locales are reported where character
decoding happens, since `wc` does more work under a UTF-8 locale than under
`LC_ALL=C`. A dash marks a row where no decoding happens and the locale cannot
matter. Times are hyperfine means with standard deviation over 10 runs, or 500
runs for the 4KB row.

| Workload | `wc` (UTF-8) | `wc` (`LC_ALL=C`) | `kz` | Speedup |
|---|---|---|---|---|
| `-m`, 100MB prose | 229.0 ms ±1.1 | 419.4 µs ±102.8 † | **3.4 ms ±0.1** | **68.2x** |
| `-L`, 1GB prose | 2.288 s ±0.023 | 1.759 s ±0.024 | **140.8 ms ±4.3** | **16.3x** |
| `-w`, 1GB prose | 2.278 s ±0.017 | 1.740 s ±0.020 | **157.4 ms ±4.5** | **14.5x** |
| default `l/w/c`, 1GB prose | 2.258 s ±0.022 | 1.744 s ±0.024 | **172.1 ms ±3.1** | **13.1x** |
| `-w`, 100MB prose | 227.3 ms ±0.8 | 172.7 ms ±0.2 | **18.1 ms ±0.4** | **12.6x** |
| default, 141 files, 100MB | 230.0 ms ±0.3 | 178.2 ms ±2.1 | **20.0 ms ±0.7** | **11.5x** |
| `-m`, 6.7MB CJK | 21.8 ms ±0.2 | 405.3 µs ±88.1 † | **1.9 ms ±0.1** | **11.3x** |
| `-w`, 100MB via stdin | 228.5 ms ±4.0 | n/a | **31.7 ms ±0.5** | **7.2x** |
| `--files0-from`, 2,972 files | 24.9 ms ±0.5 | n/a | **5.3 ms ±0.3** | **4.67x** |
| `-l`, 1GB prose | 52.6 ms ±1.0 | n/a | **18.5 ms ±0.3** | **2.8x** |
| recursive tree walk | 35.5 ms ±0.9 | n/a | **25.5 ms ±0.6** | **1.39x** |
| `-l`, 100MB via stdin | 7.6 ms ±0.4 | n/a | 8.9 ms ±0.1 | *`wc` wins, 1.16x* |
| 4KB file | 428.8 µs ±34.8 | n/a | 534.8 µs ±49.8 | *`wc` wins, 1.25x* |

† Under `LC_ALL=C`, `wc -m` answers a different question. See below.

### About the `-m` row

68x is the largest number here, so it deserves the most scrutiny. The C-locale
figure makes it look rigged at first glance: 229.0 ms against 419.4 µs. The
difference is which tool changes its answer.

| | time | result |
|---|---|---|
| `wc -m` (UTF-8) | 229.0 ms | 102,891,699 |
| `wc -m` (`LC_ALL=C`) | 419.4 µs | 104,329,273, the same as `wc -c`, a byte count |
| `kz -m` | 3.4 ms | 102,891,699, the same as `wc` UTF-8 |

Under `LC_ALL=C`, `wc -m` stops counting characters and counts bytes. `kz`
returns the UTF-8 character count in every locale, so the UTF-8 column is the
fair comparison. This is the reverse of the old `-L` bug: there `kz` was the
tool doing less work and giving the wrong answer, and the comparison flattered
us. Here `kz` does the full work and `wc` is the one that degrades.

The 68x splits into two parts, measured with `RAYON_NUM_THREADS`:

| | time | |
|---|---|---|
| `wc -m`, single-threaded | 229.0 ms | |
| `kz -m`, 1 thread | 18.5 ms | about 12x, from the algorithm |
| `kz -m`, 16 threads | 3.1 ms | about 6x, from the cores |

The 12x is the claim worth making, because it survives on one core. GNU `wc -m`
walks the buffer one character at a time through `mbrtowc`, carrying decoder
state. `kz` validates UTF-8 over a flat byte range and counts non-continuation
bytes, which vectorizes. The other 6x is just cores.

## Where `kz` does not win

Stated up front, because these are the cases people will try.

- **Small inputs.** On a 4KB file `wc` is 1.25x faster, and this cannot be
  fixed: it is process startup, not counting. `kz --version`, which opens no
  file, is already about 140µs behind (measured twice, at 500µs vs 366µs and
  554µs vs 410µs). `rayon` is not the cause, since `RAYON_NUM_THREADS=1`
  changes nothing and the pool is initialized lazily. What is left is binary
  size, 2.1MB against 72KB. Below roughly 100KB, use `wc`.
- **`-l` from stdin.** Close to parity, with `wc` ahead by 1.16x. Both tools
  now sit on the pipe-read floor of about 8ms per 100MB, and there is no win
  available above it. This is an improvement from 2.6x, and parity was the
  goal.
- **`-c` (bytes).** Both tools stat the file. There is nothing to optimize.
- **`-m` under `LC_ALL=C`.** In the C locale `wc -m` becomes a byte count and
  returns almost instantly. The comparison above is against `wc` in a UTF-8
  locale, which is the like-for-like one.
- **Single-core machines.** Speedups scale with core count, so on one core
  expect parity rather than a win. `-m` is the exception, since its 12x comes
  from the algorithm and survives with `RAYON_NUM_THREADS=1`.

## Streaming stdin

`-l` and `-c` from stdin are counted one chunk at a time and never held in
memory. Peak memory is one 256KB chunk instead of the whole stream, which
matters more than the milliseconds. Under `ulimit -v 61440`, a 60MB address
space, 100MB through a pipe counts correctly. The previous build died with
`kz: stdin: out of memory`.

This applies to `-l` and `-c` only. Under the same limit `kz -w` still exits
with `kz: stdin: out of memory`, because it has to buffer. That is the gating
working as intended, not a bug.

Every other count reads stdin to completion first. Streaming is limited to
counts that split cleanly at any read boundary, which means newlines and bytes.
Nothing else does: `-w` needs to know whether the previous chunk ended inside a
word, `-m` can be handed a UTF-8 sequence split down the middle, and `-L`,
`--stats` and `--histogram` can each be handed half a line. Those paths keep
their existing boundary-aware chunking, so the problem is avoided rather than
handled.

## Allocator

`kz` uses the system allocator. `mimalloc` was measured and removed. It cost
about 175µs of startup and lost 10 to 20 percent on single-file counts, because
`kz` memory-maps its input and barely allocates in the hot path, so the arena
setup has nothing to pay for itself with.

One workload pays for that choice. `--unique` is 7 to 8 percent slower without
`mimalloc` (207.6ms against 192.7ms on 100MB), since it builds and merges a
`HashSet` per chunk. `--stats`, `--histogram` and the recursive walk are
unaffected. The trade is deliberate.

## Why it is faster

`kz` memory-maps the input, splits it into chunks on boundaries that are safe
for the count being performed, and counts them across a `rayon` thread pool,
using `memchr` for the scanning inner loops. GNU `wc` is single-threaded.

The win is spending cores rather than a cleverer serial algorithm, with one
exception: `-m` keeps about a 12x advantage on a single thread, for the reason
given above. Everywhere else the speedup scales with core count and disappears
on one core.

## Corrections

Two numbers previously reported here were wrong. Both were measured on
generated pure-ASCII input.

- **`-L`, 87.8x.** `kz -L` reported the longest line in bytes, while `wc -L`
  reports it in display columns. Most of that speedup was `kz` doing less work,
  and the two tools disagreed on any input with an accented character, a
  typographic quote, a dash, a tab or CJK text. `-L` is now width-aware and
  agrees with `wc`.
- **`-w`, 10.4x.** That held only on pure-ASCII input. On real prose the same
  build was 4.6x slower than `wc`, because encoding auto-detection scanned the
  whole file serially before any counting started. Detection has been removed.
  `wc` does not sniff its input either.

Neither number should be cited. The corpora above exist so that this kind of
error is caught by the benchmark rather than by a reader.

Two further rows moved down, for a different reason. The code corpus was
originally whatever happened to be in `~/.cargo/registry/src` on one machine,
which nobody else could rebuild. Switching it to `cargo vendor` from this
repository's lockfile made it reproducible and also made it smaller, so
`--files0-from` went from 5.3x to 4.67x and the recursive walk from 1.7x to
1.39x. The earlier numbers were not wrong, but they were not checkable. Lower
and checkable is worth more.
