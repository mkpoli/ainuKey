# data/

## `ngrams.bin`

A compact n-gram suggestion table (unigram frequencies + bigram + trigram
next-word models) consumed by `src/suggest.rs`. Next-word prediction uses
trigram → bigram → unigram backoff.

- **Source:** aggregate word/bigram/trigram **counts** derived from the
  [`ainu-corpora`](https://github.com/mkpoli/ainu-corpora) corpus (~196k aligned
  Ainu sentences). Only frequency counts are stored — not the source texts. The
  project owner has cleared distribution of these aggregate counts for the IME's
  suggestion feature, so the table is **committed and shipped**.
- **Format:** little-endian, documented at the top of `src/suggest.rs` and
  written by `tools/build_ngrams.py`.
- **Words** are canonical lowercase Latin; the candidate UI converts them to
  katakana for display via `ainconv`.

### Build

`build.rs` copies this committed table into `OUT_DIR` for `include_bytes!`. If it
is ever absent (e.g. deleted locally), an empty table is embedded instead, so the
crate still builds — just with suggestions disabled.

### Regenerate

```sh
uv run tools/build_ngrams.py            # reads ../ainu-corpora/data.jsonl
# or:  uv run tools/build_ngrams.py --corpus PATH --out data/ngrams.bin
```

Regenerate and re-commit when the corpus or the pruning knobs in
`build_ngrams.py` change.
