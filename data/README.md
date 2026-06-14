# data/

## `ngrams.bin`

A compact n-gram suggestion table embedded into the IME (`src/suggest.rs` via
`include_bytes!`). It holds a unigram frequency list (default suggestions) and a
bigram model (previous word → top next words) used for next-word prediction.

- **Source:** derived word/bigram **counts** from the
  [`ainu-corpora`](https://github.com/mkpoli/ainu-corpora) corpus
  (~196k aligned Ainu sentences). Only aggregate frequencies are stored, not the
  source texts. See that repository for the corpus license and attribution.
- **Format:** little-endian, documented at the top of `src/suggest.rs` and
  written by `tools/build_ngrams.py`.
- **Words** are canonical lowercase Latin; the candidate UI converts them to
  katakana for display via `ainconv`.

### Regenerate

```sh
uv run tools/build_ngrams.py            # reads ../ainu-corpora/data.jsonl
# or:  uv run tools/build_ngrams.py --corpus PATH --out data/ngrams.bin
```

The file is generated and committed (the corpus is not vendored here), so it is
**not** rebuilt during `cargo build`. Regenerate and commit when the corpus or
the pruning knobs in `build_ngrams.py` change.
