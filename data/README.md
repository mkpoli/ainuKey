# data/

## `ngrams.bin` — generated, **not committed**

A compact n-gram suggestion table (unigram frequencies + bigram + trigram
next-word models) consumed by `src/suggest.rs`. Next-word prediction uses
trigram → bigram → unigram backoff.

> [!IMPORTANT]
> **Rights.** This table is **derived from the rights-restricted
> [`ainu-corpora`](https://github.com/mkpoli/ainu-corpora)** (its `data.jsonl`
> is itself gitignored/private; the underlying texts belong to their
> rights-holders). Per the same posture as
> [`ainu-llm`](https://github.com/mkpoli/ainu-llm) and `ainu-tts`, any
> corpus-derived artifact is **private by default** and **releasing it needs a
> rights/ethics review first**. So `ngrams.bin` is **gitignored and never
> committed**, and is **not** shipped in public release artifacts pending review.

### How the build works

`build.rs` provides the table to `OUT_DIR` for `include_bytes!`:

- If a locally generated `data/ngrams.bin` is present (you ran the generator
  with the private corpus), it is embedded → suggestions work.
- Otherwise (public build / CI without the corpus) an **empty** table is
  embedded → the IME builds and runs with suggestions simply disabled.

This keeps corpus-derived data out of the public repo and out of public builds,
while letting a local build (or a future, rights-cleared release) use the real
table.

### Regenerate (local, with the private corpus)

```sh
uv run tools/build_ngrams.py            # reads ../ainu-corpora/data.jsonl
# or:  uv run tools/build_ngrams.py --corpus PATH --out data/ngrams.bin
```

The format is little-endian, documented at the top of `src/suggest.rs` and
written by `tools/build_ngrams.py`. Words are canonical lowercase Latin; the
candidate UI converts them to katakana for display via `ainconv`.
