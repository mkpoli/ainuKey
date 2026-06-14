#!/usr/bin/env python3
"""Build the ainuKey suggestion n-gram table from the ainu-corpora corpus.

Reads aligned Ainu sentences, tokenizes the Latin text, and emits a compact
binary table (`data/ngrams.bin`) that the IME embeds via `include_bytes!`:

  - a unigram frequency list (default / fallback suggestions), and
  - a bigram model: previous word -> top-K likely next words.

Words are stored in canonical lowercase Latin (the IME's internal form); the
IME converts them to katakana for display via `ainconv`.

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

DEFAULT_CORPUS = Path("../ainu-corpora/data.jsonl")
DEFAULT_OUT = Path(__file__).resolve().parent.parent / "data" / "ngrams.bin"

MAGIC = b"AKNG"
VERSION = 1

# Pruning knobs (tuned for a small embedded table with useful coverage).
MAX_UNIGRAMS = 4000  # default-suggestion vocabulary
MIN_CONTEXT_COUNT = 3  # drop rare previous-word contexts
TOP_K_NEXT = 8  # next-words kept per context
MIN_NEXT_COUNT = 2  # drop rare continuations
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
            # contains stray characters (digits, foreign letters) -> skip
            continue
        if len(t.encode("utf-8")) > MAX_WORD_BYTES:
            continue
        out.append(t)
    return out


def build(corpus: Path) -> tuple[Counter, dict[str, Counter]]:
    uni: Counter = Counter()
    nexts: dict[str, Counter] = {}
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
                nexts.setdefault(a, Counter())[b] += 1
    print(f"records: {n:,}  vocab: {len(uni):,}  contexts: {len(nexts):,}", file=sys.stderr)
    return uni, nexts


def write_str(buf: bytearray, s: str) -> None:
    b = s.encode("utf-8")
    assert len(b) < 256
    buf.append(len(b))
    buf.extend(b)


def serialize(uni: Counter, nexts: dict[str, Counter]) -> bytes:
    buf = bytearray()
    buf.extend(MAGIC)
    buf.extend(struct.pack("<I", VERSION))

    # --- unigrams (top N, descending) ---
    top_uni = uni.most_common(MAX_UNIGRAMS)
    buf.extend(struct.pack("<I", len(top_uni)))
    for word, count in top_uni:
        write_str(buf, word)
        buf.extend(struct.pack("<I", count))

    # --- bigram contexts ---
    ctx_entries = []
    for ctx, cnt in nexts.items():
        if sum(cnt.values()) < MIN_CONTEXT_COUNT:
            continue
        top = [(w, c) for w, c in cnt.most_common(TOP_K_NEXT) if c >= MIN_NEXT_COUNT]
        if top:
            ctx_entries.append((ctx, top))
    ctx_entries.sort(key=lambda e: e[0])  # sorted for optional binary search

    buf.extend(struct.pack("<I", len(ctx_entries)))
    for ctx, top in ctx_entries:
        write_str(buf, ctx)
        buf.append(len(top))
        for word, count in top:
            write_str(buf, word)
            buf.extend(struct.pack("<I", count))

    print(
        f"serialized: {len(top_uni):,} unigrams, {len(ctx_entries):,} contexts, "
        f"{len(buf):,} bytes",
        file=sys.stderr,
    )
    return bytes(buf)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    args = ap.parse_args()

    uni, nexts = build(args.corpus)
    data = serialize(uni, nexts)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_bytes(data)
    print(f"wrote {args.out} ({len(data):,} bytes)", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
