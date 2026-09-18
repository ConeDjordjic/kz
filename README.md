# kz

[![Crates.io](https://img.shields.io/crates/v/kz-cli)](https://crates.io/crates/kz-cli)
[![CI](https://github.com/ConeDjordjic/kz/actions/workflows/ci.yml/badge.svg)](https://github.com/ConeDjordjic/kz/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Fast `wc` replacement.

Installed as the `kz` command (crate name: `kz-cli`).

## Install

```bash
cargo install kz-cli
```

Or from source:
```bash
git clone https://github.com/ConeDjordjic/kz
cd kz
cargo install --path .
```

## Performance

Measured on real text: concatenated Project Gutenberg prose, a CJK corpus and
real source trees, not generated ASCII. AMD Ryzen 7 7800X3D (8C/16T), against
GNU coreutils 9.11, measured with hyperfine. Full method, corpora and the cases
where `wc` wins are in [BENCHMARKS.md](BENCHMARKS.md).

```
Characters (-m), 100MB:   68x faster (3.4ms vs 229ms)
Longest line (-L), 1GB:   16x faster (141ms vs 2.29s)
Word count (-w), 1GB:     15x faster (157ms vs 2.28s)
All counts, 1GB:          13x faster (172ms vs 2.26s)
141 files, 100MB:         11x faster (20ms vs 230ms)
Line count (-l), 1GB:    2.8x faster (19ms vs 53ms)
```

`kz` wins 11 of 13 measured workloads. Speedups scale with core count. `-m` is
the exception: about 12x of it survives on a single thread, because `wc`
decodes one character at a time through `mbrtowc` while `kz` validates UTF-8
over a flat byte range.

It loses two, both worth knowing before you install it. On a 4KB file `wc` is
1.25x faster, and that gap cannot be closed, because it is process startup and
not counting: `kz --version`, which opens no file, is already about 140µs
behind. Below roughly 100KB, use `wc`. Piping `-l` from stdin is close to
parity, with `wc` ahead by 1.16x, because both tools sit on the pipe-read
floor.

## Agreement with `wc`

`kz` is a drop-in replacement, so it aims to report exactly what `wc` reports.
`-l`, `-w`, `-c`, `-m` and `-L` are verified against GNU coreutils on the
benchmark corpora, where all five counts are identical on 100MB of real prose
and on a 96% non-ASCII CJK corpus. They are also checked on accented Latin,
legacy-encoded input (latin-1, cp1252, Shift_JIS, GBK, Big5) and invalid UTF-8.

Two rules follow from that, and both were once the other way around:

- **No encoding auto-detection.** `wc` does not sniff its input, so `kz` does
  not either. Bytes are read as UTF-8. `--encoding` decodes explicitly and is
  the only path that does.
- **`-m` never falls back to bytes.** Like `wc -m`, it counts one character per
  valid UTF-8 sequence, and nothing for a byte that is not part of one.

## Usage

```bash
kz file.txt              # lines, words, bytes
kz -l file.txt           # lines
kz -w file.txt           # words
kz -c file.txt           # bytes
kz -m file.txt           # characters (UTF-8)
kz -L file.txt           # longest line, in display columns
kz -lwc file.txt         # combine flags
kz -w *.txt              # multiple files
cat file.txt | kz -w     # stdin
kz --pattern "foo" file  # count pattern
```

## Options

```
-l, --lines              line count
-w, --words              word count
-c, --bytes              byte count
-m, --chars              character count (UTF-8)
-L, --max-line-length    longest line, in display columns (like wc -L)
-b, --blank-lines        blank line count
-r, --recursive          recurse directories
-v, --verbose            show warnings
--unique                 unique word count
--pattern <PAT>          count pattern occurrences
--stats                  show statistics (mean, median, std dev)
--histogram              line length distribution
--json                   JSON output
--timing                 show processing time
--total-only             only show total (skip per-file output)
--progress               show progress
--code                   skip comments (// /* # -- """)
--markdown               skip code blocks
--exclude <PAT>          exclude files matching pattern
--encoding <ENC>         decode input as ENCODING before counting
--files0-from <FILE>     read null-terminated filenames
--generate-completion    shell completions (bash/zsh/fish/powershell)
```

## Examples

```bash
# Recursive with exclusions
kz -r --exclude "*.min.js" --exclude "node_modules/*" src/

# Statistics
kz --stats file.txt

# JSON output with timing
kz --json --timing file.txt

# Code lines only (skip comments)
kz --code -l src/*.rs

# Markdown text only (skip code blocks)
kz --markdown -w README.md

# Decode a legacy-encoded file before counting. Without this, bytes are read
# as UTF-8, which is what wc does in a UTF-8 locale.
kz --encoding iso-8859-1 legacy.txt

# Progress for large operations
kz --progress -r ~/projects/

# Blank lines count
kz -b src/*.rs

# Total only (no per-file output)
kz --total-only -r src/
```

## Shell Completions

```bash
kz --generate-completion bash > ~/.local/share/bash-completion/completions/kz
kz --generate-completion zsh > ~/.zfunc/_kz
kz --generate-completion fish > ~/.config/fish/completions/kz.fish
```

## Implementation

- Parallel processing (Rayon, 1MB chunks)
- Streaming stdin for `-l` and `-c` (bounded memory, one 256KB chunk)
- Memory-mapped I/O
- SIMD pattern matching (memchr)
- UTF-8 aware with proper chunk boundaries
- Unicode whitespace detection
- CRLF normalization
- Explicit encoding conversion via `--encoding`
- Binary file detection

## License

MIT
