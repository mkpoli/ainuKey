#!/usr/bin/env python3
"""Build the ainuKey suggestion n-gram table from the ainu-corpora corpus.

Reads aligned Ainu sentences, tokenizes the Latin text, and emits a compact
binary table (`data/ngrams.bin`) that the IME embeds via `include_bytes!`:

  - a unigram frequency list (default / fallback suggestions),
  - a bigram model:  previous word          -> top-K next words, and
  - a trigram model: (prev2 prev1) context  -> top-K next words.

The IME predicts the next word with trigram → bigram → unigram backoff. Words
are canonical lowercase Latin (the IME's internal form); the IME converts them
to katakana for display via `ainconv`.

Usage:
    uv run tools/build_ngrams.py [--corpus PATH] [--out PATH]

The table is regenerated manually and committed; it is NOT built during
`cargo build`.
"""
from __future__ import annotations

import argparse
import json
import re
import struct
import sys
from collections import Counter
from pathlib import Path

DEFAULT_CORPUS = Path("/home/mkpoli/projects/Ainu/ainu-corpora/data.jsonl")
DEFAULT_OUT = Path(__file__).resolve().parent.parent / "data" / "ngrams.bin"

MAGIC = b"AKNG"
VERSION = 2

# Pruning knobs (tuned for a small embedded table with useful coverage).
MAX_UNIGRAMS = 4000  # default-suggestion vocabulary
MIN_CONTEXT_COUNT = 3  # drop rare bigram contexts
TOP_K_NEXT = 8  # next-words kept per bigram context
MIN_NEXT_COUNT = 2  # drop rare bigram continuations
MIN_TRI_CONTEXT = 4  # drop rare trigram contexts (sparser -> stricter)
TOP_K_TRI = 6  # next-words kept per trigram context
MIN_TRI_NEXT = 2  # drop rare trigram continuations
MAX_WORD_BYTES = 40  # skip pathological tokens

PAREN = re.compile(r"\(([^)]*)\)")
KEEP = set("abcdefghijklmnopqrstuvwxyzáíúéó'’=-")
STRIP = ".,!?;:\"“”«»…()[]{}<>/\\|*"


def tokenize(text: str) -> list[str]:
    """Lowercase, fold optional-sound parens (a(n)->an), split on whitespace,
    strip surrounding punctuation, keep intra-word ' ’ = - (Ainu affix/clitic)."""
    text = PAREN.sub(r"\1", text.lower())
    out: list[str] = []
    for raw in text.split():
        t = raw.strip(STRIP)
        if not t or not any(c.isalpha() for c in t):
            continue
        if any(c not in KEEP for c in t):
            continue  # stray characters (digits, foreign letters) -> skip
        if len(t.encode("utf-8")) > MAX_WORD_BYTES:
            continue
        out.append(t)
    return out


def build(corpus: Path):
    uni: Counter = Counter()
    bi: dict[str, Counter] = {}
    tri: dict[tuple[str, str], Counter] = {}
    n = 0
    with corpus.open(encoding="utf-8") as f:
        for line in f:
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                continue
            text = rec.get("text") or ""
            if not text:
                continue
            n += 1
            toks = tokenize(text)
            uni.update(toks)
            for a, b in zip(toks, toks[1:]):
                bi.setdefault(a, Counter())[b] += 1
            for a, b, c in zip(toks, toks[1:], toks[2:]):
                tri.setdefault((a, b), Counter())[c] += 1
    print(
        f"records: {n:,}  vocab: {len(uni):,}  "
        f"bigram-ctx: {len(bi):,}  trigram-ctx: {len(tri):,}",
        file=sys.stderr,
    )
    return uni, bi, tri


def write_str(buf: bytearray, s: str) -> None:
    b = s.encode("utf-8")
    assert len(b) < 256, s
    buf.append(len(b))
    buf.extend(b)


def write_context_section(buf: bytearray, entries: list[tuple[str, list[tuple[str, int]]]]) -> None:
    buf.extend(struct.pack("<I", len(entries)))
    for ctx, top in entries:
        write_str(buf, ctx)
        buf.append(len(top))
        for word, count in top:
            write_str(buf, word)
            buf.extend(struct.pack("<I", count))


def prune(
    contexts, min_context: int, top_k: int, min_next: int, key=lambda k: k
) -> list[tuple[str, list[tuple[str, int]]]]:
    out = []
    for ctx, cnt in contexts.items():
        if sum(cnt.values()) < min_context:
            continue
        top = [(w, c) for w, c in cnt.most_common(top_k) if c >= min_next]
        if top:
            out.append((key(ctx), top))
    out.sort(key=lambda e: e[0])
    return out


def serialize(uni: Counter, bi, tri) -> bytes:
    buf = bytearray()
    buf.extend(MAGIC)
    buf.extend(struct.pack("<I", VERSION))

    # unigrams (top N, descending)
    top_uni = uni.most_common(MAX_UNIGRAMS)
    buf.extend(struct.pack("<I", len(top_uni)))
    for word, count in top_uni:
        write_str(buf, word)
        buf.extend(struct.pack("<I", count))

    # bigram contexts: "prev" -> nexts
    bi_entries = prune(bi, MIN_CONTEXT_COUNT, TOP_K_NEXT, MIN_NEXT_COUNT)
    write_context_section(buf, bi_entries)

    # trigram contexts: "prev2 prev1" -> nexts
    tri_entries = prune(
        tri, MIN_TRI_CONTEXT, TOP_K_TRI, MIN_TRI_NEXT, key=lambda k: f"{k[0]} {k[1]}"
    )
    write_context_section(buf, tri_entries)

    print(
        f"serialized: {len(top_uni):,} unigrams, {len(bi_entries):,} bigram-ctx, "
        f"{len(tri_entries):,} trigram-ctx, {len(buf):,} bytes",
        file=sys.stderr,
    )
    return bytes(buf)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = ap.parse_args()

    uni, bi, tri = build(args.corpus)
    data = serialize(uni, bi, tri)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_bytes(data)
    print(f"wrote {args.out} ({len(data):,} bytes)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
