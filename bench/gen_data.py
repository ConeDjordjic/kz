#!/usr/bin/env python3
"""Generate reproducible corpora for the kz benchmark suite."""
import os, random, sys

OUT = sys.argv[1] if len(sys.argv) > 1 else "bench/data"

def vocab(seed, n=60000):
    r = random.Random(seed)
    return r, [''.join(r.choices('abcdefghijklmnopqrstuvwxyz', k=r.randint(2, 11))) for _ in range(n)]

def big_ascii(path, target=1_000_000_000):
    r, words = vocab(1337)
    with open(path, 'w') as f:
        buf, size = [], 0
        while size < target:
            line = ' '.join(r.choices(words, k=r.randint(6, 18))) + '\n'
            buf.append(line); size += len(line)
            if len(buf) > 20000:
                f.write(''.join(buf)); buf = []
        f.write(''.join(buf))

def big_utf8(path, target=500_000_000):
    """Mixed-script prose: a byte count and a display width disagree here.

    `wc -L` reports display columns, so a corpus that is pure ASCII can never
    catch a `-L` implementation that measures bytes. This one plants the
    widest line as CJK, where bytes are 3x the column count.
    """
    r, words = vocab(2024)
    accents = str.maketrans('aeiounc', 'áéíóúñç')
    cjk = '日本語のテキストと漢字混在'
    with open(path, 'w') as f:
        buf, size = [], 0
        while size < target:
            line = ' '.join(r.choices(words, k=r.randint(6, 18)))
            roll = r.random()
            if roll < 0.55:
                # accented Latin: 2 bytes per accented char, 1 column
                line = ''.join(c.translate(accents) if r.random() < 0.15 else c for c in line)
            elif roll < 0.70:
                line = line + ' — “' + r.choice(words) + '”'
            elif roll < 0.80:
                line = r.choice(words) + ' ' + cjk[:r.randint(2, 13)] + ' ' + line
            buf.append(line + '\n'); size += len(line.encode()) + 1
            if len(buf) > 20000:
                f.write(''.join(buf)); buf = []
        f.write(''.join(buf))
        # widest line in the file, and deliberately not ASCII: 320 columns
        # in 480 bytes. A byte-length `-L` overshoots it by 160.
        f.write(cjk[:10] * 16 + '\n')


def corpus(d, count=500):
    r, words = vocab(7, 20000)
    os.makedirs(d, exist_ok=True)
    for i in range(count):
        with open(f'{d}/f{i:04d}.txt', 'w') as f:
            for _ in range(r.randint(2000, 8000)):
                f.write(' '.join(r.choices(words, k=r.randint(6, 18))) + '\n')

def mb(name, default):
    """Corpus sizes are overridable so CI can generate a small one."""
    return int(os.environ.get(name, default))

if __name__ == '__main__':
    ascii_bytes = mb('KZ_ASCII_BYTES', 1_000_000_000)
    utf8_bytes = mb('KZ_UTF8_BYTES', 500_000_000)
    corpus_files = mb('KZ_CORPUS_FILES', 500)

    os.makedirs(OUT, exist_ok=True)
    print(f'generating big-ascii.txt ({ascii_bytes:,} bytes)...')
    big_ascii(f'{OUT}/big-ascii.txt', ascii_bytes)
    print(f'generating big-utf8.txt ({utf8_bytes:,} bytes)...')
    big_utf8(f'{OUT}/big-utf8.txt', utf8_bytes)
    if corpus_files:
        print(f'generating corpus/ ({corpus_files} files)...')
        corpus(f'{OUT}/corpus', corpus_files)
    with open(f'{OUT}/big-ascii.txt','rb') as s, open(f'{OUT}/small.txt','wb') as d:
        d.write(s.read(4096))
    print('done ->', OUT)
