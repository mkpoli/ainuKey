#!/usr/bin/env python3
"""Benchmark the ainuKey suggestion engine — and measure whether
*area-specified* (per-collection) models improve the writing experience.

The metrics are the ones that actually describe "good to write with", computed
on a **document-level held-out test split** (a whole document is entirely in
train or test, so completions can't leak across the split):

  * KSR  — Keystroke-Savings Rate: of all the letters you'd type, what fraction
           the IME lets you skip by accepting a completion. The headline number.
  * hit@k — next-word prediction: is the actual next word in the top-k the
           context model predicts (before you type it)? k = 1/3/5/9.
  * MRR  — mean reciprocal rank of the next word in that prediction.
  * fire — fraction of positions where the context model predicts anything.
  * OOV  — fraction of next words the model has never seen.

For each area it compares three engines on that area's test text:
  * global — the single model trained on the whole corpus (today's shipped model)
  * area   — a model trained only on that area
  * blend  — the area model boosted on top of global (in-domain words/collocations
             rank higher, global coverage retained) → "the user told us the domain"

Everything uses tools/ngram_lib.py, so the simulation matches the Rust IME.

Usage:
    uv run tools/bench_ngrams.py [--corpus PATH] [--min-area-tokens N]
                                 [--test-frac 0.15] [--sample-frac F]
                                 [--max-cand 9] [--out tools/bench_report.md]
"""
from __future__ import annotations

import argparse
import hashlib
import sys
from collections import defaultdict
from itertools import chain
from pathlib import Path

from ngram_lib import (
    BlendModel,
    Model,
    PruneCfg,
    candidate_list,
    iter_records,
    tokenize,
)

DEFAULT_CORPUS = Path("/home/mkpoli/projects/Ainu/ainu-corpora/data.jsonl")
DEFAULT_OUT = Path(__file__).resolve().parent / "bench_report.md"
AREA_FIELD = "collection_lv1"
OTHER = "∅ (uncategorized)"


def split_is_test(key: str, test_frac: float) -> bool:
    """Deterministic document-level split (stable across runs / Python's salted
    hash). A whole document lands wholly in train or wholly in test."""
    h = hashlib.blake2b(key.encode("utf-8"), digest_size=8).digest()
    return (int.from_bytes(h, "big") % 10000) < int(test_frac * 10000)


def load(corpus: Path, test_frac: float):
    """Return {area: {"train": [tokens...], "test": [tokens...]}}."""
    data: dict[str, dict[str, list[list[str]]]] = defaultdict(
        lambda: {"train": [], "test": []}
    )
    n = 0
    for rec in iter_records(corpus):
        text = rec.get("text") or ""
        if not text:
            continue
        toks = tokenize(text)
        if not toks:
            continue
        n += 1
        area = rec.get(AREA_FIELD) or OTHER
        # Keep a whole document on one side; fall back to the record id.
        key = rec.get("document") or f"id:{rec.get('id')}"
        bucket = "test" if split_is_test(str(key), test_frac) else "train"
        data[area][bucket].append(toks)
    print(f"loaded {n:,} non-empty records across {len(data):,} areas", file=sys.stderr)
    return data


# --- metric accumulation ----------------------------------------------------


class Metrics:
    KS = (1, 3, 5, 9)

    def __init__(self) -> None:
        self.positions = 0  # next-word prediction positions (prev1 known)
        self.fire = 0  # positions where context predicted >=1 word
        self.hit = {k: 0 for k in self.KS}
        self.rr = 0.0  # reciprocal-rank sum
        self.oov = 0
        # keystroke savings
        self.chars_plain = 0
        self.chars_saved = 0
        self.chars_saved_invocab = 0
        self.chars_plain_invocab = 0

    def eval_engine(self, engine, test_docs, max_cand: int, sample_frac: float):
        seen = 0
        for toks in test_docs:
            for i, target in enumerate(toks):
                prev1 = toks[i - 1] if i >= 1 else None
                prev2 = toks[i - 2] if i >= 2 else None
                in_vocab = target in engine

                # --- keystroke savings (simulate typing `target`) ---
                L = len(target)
                if L >= 2:
                    seen += 1
                    sample = (seen % 10000) < int(sample_frac * 10000)
                    if sample:
                        self.chars_plain += L
                        if in_vocab:
                            self.chars_plain_invocab += L
                        best = L  # cost with IME; default = type it fully
                        for p in range(1, L):
                            cands = candidate_list(
                                engine, prev2, prev1, target[:p], max_cand
                            )
                            if target in cands[1:]:
                                best = p + 1  # p typed + 1 selection key
                                break
                        saved = max(0, L - best)
                        self.chars_saved += saved
                        if in_vocab:
                            self.chars_saved_invocab += saved

                # --- next-word prediction (needs a left context) ---
                if prev1 is None:
                    continue
                self.positions += 1
                if not in_vocab:
                    self.oov += 1
                scores = engine.predict_scores(prev2, prev1)
                if scores:
                    self.fire += 1
                ranked = sorted(scores.items(), key=lambda kv: (-kv[1], kv[0]))
                rank = next(
                    (r for r, (w, _) in enumerate(ranked, 1) if w == target), None
                )
                if rank is not None:
                    self.rr += 1.0 / rank
                    for k in self.KS:
                        if rank <= k:
                            self.hit[k] += 1
        return self

    def row(self) -> dict[str, float]:
        p = max(1, self.positions)
        cp = max(1, self.chars_plain)
        cpv = max(1, self.chars_plain_invocab)
        return {
            "KSR": self.chars_saved / cp,
            "KSR_iv": self.chars_saved_invocab / cpv,
            "hit@1": self.hit[1] / p,
            "hit@3": self.hit[3] / p,
            "hit@5": self.hit[5] / p,
            "hit@9": self.hit[9] / p,
            "MRR": self.rr / p,
            "fire": self.fire / p,
            "OOV": self.oov / p,
        }


# --- reporting --------------------------------------------------------------

COLS = ["KSR", "KSR_iv", "hit@1", "hit@3", "hit@5", "MRR", "fire", "OOV"]


def fmt_row(label: str, r: dict[str, float], extra: str = "") -> str:
    cells = " | ".join(f"{r[c]*100:5.1f}" for c in COLS)
    return f"| {label:<22} | {cells} |{extra}"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    ap.add_argument("--test-frac", type=float, default=0.15)
    ap.add_argument("--min-area-tokens", type=int, default=20000)
    ap.add_argument(
        "--sample-frac",
        type=float,
        default=1.0,
        help="evaluate KSR on this fraction of words (speed knob; metrics are stable)",
    )
    ap.add_argument("--max-cand", type=int, default=9)
    args = ap.parse_args()

    if not args.corpus.exists():
        print(f"corpus not found: {args.corpus} (pass --corpus PATH)", file=sys.stderr)
        return 1

    data = load(args.corpus, args.test_frac)

    # Global model: every area's train docs, pruned exactly like the shipped table.
    print("training global model…", file=sys.stderr)
    global_train = chain.from_iterable(d["train"] for d in data.values())
    gmodel = Model.train(global_train, PruneCfg.production())

    def tok_count(docs):
        return sum(len(t) for t in docs)

    # Areas big enough to train a meaningful own model.
    areas = sorted(
        (a for a, d in data.items()
         if a != OTHER and tok_count(d["train"]) >= args.min_area_tokens),
        key=lambda a: -tok_count(data[a]["train"]),
    )
    print(f"area models: {len(areas)} (>= {args.min_area_tokens:,} train tokens)",
          file=sys.stderr)

    lines: list[str] = []
    lines.append("# ainuKey prediction benchmark\n")
    lines.append(
        f"Document-level split: test_frac={args.test_frac}, "
        f"max_cand={args.max_cand}, sample_frac={args.sample_frac}. "
        f"All numbers are percentages. Headline = **KSR** (keystroke savings); "
        f"`KSR_iv` = KSR over in-vocab words only.\n"
    )
    header = "| " + "engine / area".ljust(22) + " | " + " | ".join(
        f"{c:>5}" for c in COLS
    ) + " |"
    sep = "|" + "-" * 24 + "|" + ("|".join(["-" * 7] * len(COLS))) + "|"

    # Overall baseline: global model over the whole test set.
    print("evaluating global over ALL test…", file=sys.stderr)
    all_test = list(chain.from_iterable(d["test"] for d in data.values()))
    overall = Metrics().eval_engine(gmodel, all_test, args.max_cand, args.sample_frac).row()
    lines.append("## Overall (whole-corpus test set)\n")
    lines.append(header)
    lines.append(sep)
    lines.append(fmt_row("global", overall))
    lines.append("")

    # Per-area: global vs area vs blend.
    summary: list[tuple[str, dict, dict, dict, int]] = []
    for area in areas:
        test_docs = data[area]["test"]
        ntok = tok_count(test_docs)
        if ntok == 0:
            continue
        print(f"  area: {area}  (test {ntok:,} tok)…", file=sys.stderr)
        amodel = Model.train(data[area]["train"], PruneCfg.light())
        blend = BlendModel(amodel, gmodel)
        g = Metrics().eval_engine(gmodel, test_docs, args.max_cand, args.sample_frac).row()
        a = Metrics().eval_engine(amodel, test_docs, args.max_cand, args.sample_frac).row()
        b = Metrics().eval_engine(blend, test_docs, args.max_cand, args.sample_frac).row()
        summary.append((area, g, a, b, ntok))

    lines.append("## Per area: global vs area-only vs area+global blend\n")
    for area, g, a, b, ntok in summary:
        d_ksr = (b["KSR"] - g["KSR"]) * 100
        d_h3 = (b["hit@3"] - g["hit@3"]) * 100
        lines.append(f"### {area}  ({ntok:,} test tokens)\n")
        lines.append(header)
        lines.append(sep)
        lines.append(fmt_row("global", g))
        lines.append(fmt_row("area-only", a))
        lines.append(fmt_row("blend (area+global)", b,
                             f"  ΔKSR {d_ksr:+.1f}, Δhit@3 {d_h3:+.1f}"))
        lines.append("")

    # Weighted aggregate of the blend's lift over global.
    if summary:
        tot = sum(n for *_, n in summary)
        w_ksr = sum((b["KSR"] - g["KSR"]) * n for _, g, _, b, n in summary) / tot * 100
        w_h3 = sum((b["hit@3"] - g["hit@3"]) * n for _, g, _, b, n in summary) / tot * 100
        w_ksr_iv = sum((b["KSR_iv"] - g["KSR_iv"]) * n for _, g, _, b, n in summary) / tot * 100
        lines.append("## Verdict (test-token-weighted blend lift over global)\n")
        lines.append(f"- **ΔKSR  {w_ksr:+.2f} pts** (in-vocab {w_ksr_iv:+.2f})")
        lines.append(f"- **Δhit@3 {w_h3:+.2f} pts**")
        lines.append(
            "\nIf the blend lift is clearly positive, area-specialization is worth "
            "shipping (per-area tables + a domain selector in settings). If it's "
            "near zero, the single global model already captures it.\n"
        )

    report = "\n".join(lines)
    print("\n" + report)
    args.out.write_text(report, encoding="utf-8")
    print(f"\nwrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
