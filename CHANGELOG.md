# Changelog

## 0.2.0

Breaking. `kz` now reports exactly what `wc` reports. Counts that disagreed
with `wc` have been corrected, so some outputs change.

### Fixed

- `-L` counts display columns, like `wc -L`, instead of bytes. Any line with an
  accented character, a typographic quote, a dash or CJK text was overcounted.
  Tabs now advance to the next multiple of 8 instead of counting as one column.
- `-m` counts one character per valid UTF-8 sequence and nothing for a byte
  that is not part of one, like `wc -m`. It no longer falls back to counting
  bytes when input fails to validate.
- Encoding auto-detection is removed. It scanned the whole input serially
  before counting, which made `-w` and `-m` several times slower than `wc` on
  real text, and it made `kz` disagree with `wc` on legacy-encoded files. `wc`
  does not sniff its input either. Use `--encoding` to decode explicitly.
- An unknown `--encoding` value now always warns, once per run, instead of
  warning only under `--verbose`.

### Removed

- `--fast`. It only affected `-m`, where it returned a byte count rather than a
  character count.

### Changed

- `-l` and `-c` from stdin are counted a chunk at a time, so memory is bounded
  by one 256KB chunk instead of the whole stream. Other counts still buffer.
- `mimalloc` is no longer used. It cost startup time and lost on most
  workloads. `--unique` is 7 to 8 percent slower as a result.

## 0.1.0

Initial release.
