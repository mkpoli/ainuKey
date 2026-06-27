#!/usr/bin/env python3
"""Prepare a compact training set for the neural next-word model — torch-free,
runs locally so we upload a few MB to the GPU box, not the 142 MB corpus.

Reuses the IME's exact tokenizer (ngram_lib) and the **same document-level
train/test split** as the n-gram benchmark (tools/bench_ngrams.py), so the
neural model is evaluated on the identical held-out data and the comparison to
the n-gram engine is apples-to-apples.

Emits (under data/neural/, gitignored — corpus-derived):
  - vocab.json   : {"itos": [...], "stoi": {...}, "specials": {...}}
  - train.txt    : one tokenized sentence per line (space-joined words)
  - test.txt     : same, held-out documents
  - meta.json    : counts / config for reproducibility

Usage:
    uv run tools/neural/prep_data.py [--corpus PATH] [--vocab-size N]
                                     [--test-frac 0.15] [--out DIR]
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
from ngram_lib import iter_records, tokenize  # noqa: E402

# $AINU_CORPUS, else a sibling `ainu-corpora` checkout (run from the repo root);
# --corpus overrides. No hardcoded home path.
DEFAULT_CORPUS = Path(os.environ.get("AINU_CORPUS", "../ainu-corpora/data.jsonl"))
DEFAULT_OUT = Path(__file__).resolve().parent.parent.parent / "data" / "neural"

# Special tokens (ids 0..2 are reserved; real words start at 3).
PAD, UNK, BOS, EOS = "<pad>", "<unk>", "<bos>", "<eos>"
SPECIALS = [PAD, UNK, BOS, EOS]


def split_is_test(key: str, test_frac: float) -> bool:
    """Identical to bench_ngrams.split_is_test — a whole document lands wholly in
    train or test, so neural and n-gram see the same held-out set."""
    h = hashlib.blake2b(key.encode("utf-8"), digest_size=8).digest()
    return (int.from_bytes(h, "big") % 10000) < int(test_frac * 10000)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    ap.add_argument("--vocab-size", type=int, default=10000,
                    help="max real words kept (rest map to <unk>)")
    ap.add_argument("--test-frac", type=float, default=0.15)
    ap.add_argument("--min-len", type=int, default=2,
                    help="skip sentences shorter than this many tokens")
    args = ap.parse_args()

    if not args.corpus.exists():
        print(f"corpus not found: {args.corpus} (pass --corpus PATH)", file=sys.stderr)
        return 1

    # First pass: collect sentences + word frequencies (train split only, so the
    # vocab never peeks at the test set).
    train_sents: list[list[str]] = []
    test_sents: list[list[str]] = []
    freq: Counter = Counter()
    for rec in iter_records(args.corpus):
        text = rec.get("text") or ""
        if not text:
            continue
        toks = tokenize(text)
        if len(toks) < args.min_len:
            continue
        key = rec.get("document") or f"id:{rec.get('id')}"
        if split_is_test(str(key), args.test_frac):
            test_sents.append(toks)
        else:
            train_sents.append(toks)
            freq.update(toks)

    # Vocab: specials + top-N words by train frequency.
    itos = list(SPECIALS) + [w for w, _ in freq.most_common(args.vocab_size)]
    stoi = {w: i for i, w in enumerate(itos)}
    coverage = sum(freq[w] for w in itos if w in freq) / max(1, sum(freq.values()))

    args.out.mkdir(parents=True, exist_ok=True)
    (args.out / "vocab.json").write_text(
        json.dumps({"itos": itos, "stoi": stoi,
                    "specials": {"pad": 0, "unk": 1, "bos": 2, "eos": 3}},
                   ensure_ascii=False),
        encoding="utf-8",
    )
    for name, sents in (("train.txt", train_sents), ("test.txt", test_sents)):
        (args.out / name).write_text(
            "\n".join(" ".join(s) for s in sents), encoding="utf-8"
        )
    meta = {
        "corpus": str(args.corpus),
        "vocab_size": len(itos),
        "vocab_requested": args.vocab_size,
        "train_sents": len(train_sents),
        "test_sents": len(test_sents),
        "train_tokens": sum(len(s) for s in train_sents),
        "test_tokens": sum(len(s) for s in test_sents),
        "test_frac": args.test_frac,
        "unigram_coverage": round(coverage, 4),
    }
    (args.out / "meta.json").write_text(json.dumps(meta, indent=2), encoding="utf-8")
    print(json.dumps(meta, indent=2))
    print(f"\nwrote {args.out}/ (vocab.json, train.txt, test.txt, meta.json)",
          file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
