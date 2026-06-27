#!/usr/bin/env python3
"""Benchmark the ainuKey suggestion engine — and measure whether
*area-specified* (per-collection) models improve the writing experience, both
with an **oracle** domain (the user told us) and with **auto-detection** (the
IME infers the domain from recent text).

Metrics, on a **document-level held-out split** (a whole document is entirely in
train or test, so completions can't leak across the split):

  * KSR  — Keystroke-Savings Rate: fraction of letters the IME lets you skip by
           accepting a completion. The headline number.
  * hit@k — next-word prediction: is the actual next word in the top-k the
           context model predicts? k = 1/3/5/9.
  * MRR  — mean reciprocal rank of that next word.
  * fire — fraction of positions where the context model predicts anything.
  * OOV  — fraction of next words the model has never seen.

Engines compared:
  * global — single whole-corpus model (today's shipped engine; the baseline)
  * area   — model trained only on that area
  * blend  — area boosted on top of global (in-domain ranks higher, coverage kept)

Regimes (whole trained-area test set):
  * global — global engine everywhere (baseline)
  * oracle — each doc uses its true area's blend (ceiling)
  * auto   — an online classifier picks the area per position from text-so-far,
             falling back to global until confident; uses that area's blend

Everything uses tools/ngram_lib.py, so the simulation matches the Rust IME.

Usage:
    uv run tools/bench_ngrams.py [--corpus PATH] [--min-area-tokens N]
                                 [--test-frac 0.15] [--sample-frac F]
                                 [--margin M] [--max-cand 9] [--out PATH]
"""
from __future__ import annotations

import argparse
import hashlib
import os
import sys
from collections import Counter, defaultdict
from itertools import chain
from pathlib import Path

from ngram_lib import (
    AreaClassifier,
    BlendModel,
    Model,
    PruneCfg,
    candidate_list,
    iter_records,
    tokenize,
)

# Corpus path: $AINU_CORPUS, else a sibling `ainu-corpora` checkout (run from the
# repo root). No hardcoded absolute/home path; override with --corpus.
DEFAULT_CORPUS = Path(os.environ.get("AINU_CORPUS", "../ainu-corpora/data.jsonl"))
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
        key = rec.get("document") or f"id:{rec.get('id')}"
        bucket = "test" if split_is_test(str(key), test_frac) else "train"
        data[area][bucket].append(toks)
    print(f"loaded {n:,} non-empty records across {len(data):,} areas", file=sys.stderr)
    return data


# --- metric accumulation ----------------------------------------------------


class Metrics:
    KS = (1, 3, 5, 9)

    def __init__(self) -> None:
        self.positions = 0
        self.fire = 0
        self.hit = {k: 0 for k in self.KS}
        self.rr = 0.0
        self.oov = 0
        self.chars_plain = 0
        self.chars_saved = 0
        self.chars_plain_invocab = 0
        self.chars_saved_invocab = 0
        self._seen = 0  # KSR sampling counter

    def score_position(self, engine, toks, i, max_cand, sample_frac):
        """Accumulate KSR + next-word metrics for one position, using `engine`."""
        target = toks[i]
        prev1 = toks[i - 1] if i >= 1 else None
        prev2 = toks[i - 2] if i >= 2 else None
        in_vocab = target in engine

        L = len(target)
        if L >= 2:
            self._seen += 1
            if (self._seen % 10000) < int(sample_frac * 10000):
                self.chars_plain += L
                if in_vocab:
                    self.chars_plain_invocab += L
                best = L  # cost with IME; default = type it fully
                for p in range(1, L):
                    cands = candidate_list(engine, prev2, prev1, target[:p], max_cand)
                    if target in cands[1:]:
                        best = p + 1  # p typed + 1 selection key
                        break
                saved = max(0, L - best)
                self.chars_saved += saved
                if in_vocab:
                    self.chars_saved_invocab += saved

        if prev1 is None:
            return
        self.positions += 1
        if not in_vocab:
            self.oov += 1
        scores = engine.predict_scores(prev2, prev1)
        if scores:
            self.fire += 1
        ranked = sorted(scores.items(), key=lambda kv: (-kv[1], kv[0]))
        rank = next((r for r, (w, _) in enumerate(ranked, 1) if w == target), None)
        if rank is not None:
            self.rr += 1.0 / rank
            for k in self.KS:
                if rank <= k:
                    self.hit[k] += 1

    def eval_engine(self, engine, test_docs, max_cand, sample_frac):
        for toks in test_docs:
            for i in range(len(toks)):
                self.score_position(engine, toks, i, max_cand, sample_frac)
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


# Cold-start buckets for classifier accuracy by word-position-in-document.
BUCKETS = [(0, 5, "1–5"), (5, 20, "6–20"), (20, 50, "21–50"), (50, 10**9, "51+")]


def eval_regimes(areas, data, gmodel, blends, classifier, margins, max_cand, sample_frac):
    """Run global / oracle / auto over the trained-area test set in ONE pass,
    evaluating the auto regime at every confidence `margin` simultaneously
    (the running classifier score is shared; only the commit threshold varies).
    Returns g_row, o_row, {margin: (a_row, stats)}."""
    g_m, o_m = Metrics(), Metrics()
    a_m = {m: Metrics() for m in margins}
    st = {m: {"correct": 0, "total": 0, "fallback": 0,
              "bc": defaultdict(int), "bt": defaultdict(int)} for m in margins}

    for area in areas:
        oracle_engine = blends.get(area, gmodel)
        for toks in data[area]["test"]:
            running: dict[str, float] = {a: 0.0 for a in areas}
            for i in range(len(toks)):
                g_m.score_position(gmodel, toks, i, max_cand, sample_frac)
                o_m.score_position(oracle_engine, toks, i, max_cand, sample_frac)
                for m in margins:
                    pred = classifier.predict(running, m)
                    s = st[m]
                    if pred is not None:
                        s["total"] += 1
                        correct = pred == area
                        s["correct"] += int(correct)
                        for lo, hi, name in BUCKETS:
                            if lo <= i < hi:
                                s["bt"][name] += 1
                                s["bc"][name] += int(correct)
                                break
                    else:
                        s["fallback"] += 1
                    engine = blends.get(pred, gmodel) if pred else gmodel
                    a_m[m].score_position(engine, toks, i, max_cand, sample_frac)
                for a in areas:
                    running[a] += classifier.token_score(a, toks[i])

    out = {}
    for m in margins:
        s = st[m]
        out[m] = (
            a_m[m].row(),
            {
                "cls_acc": s["correct"] / max(1, s["total"]),
                "cls_fallback_rate": s["fallback"] / max(1, s["total"] + s["fallback"]),
                "buckets": [
                    (name, s["bc"][name] / max(1, s["bt"][name]), s["bt"][name])
                    for _, _, name in BUCKETS
                ],
            },
        )
    return g_m.row(), o_m.row(), out


# --- reporting --------------------------------------------------------------

COLS = ["KSR", "KSR_iv", "hit@1", "hit@3", "hit@5", "MRR", "fire", "OOV"]
HEADER = "| " + "engine / area".ljust(22) + " | " + " | ".join(f"{c:>5}" for c in COLS) + " |"
SEP = "|" + "-" * 24 + "|" + ("|".join(["-" * 7] * len(COLS))) + "|"


def fmt_row(label: str, r: dict[str, float], extra: str = "") -> str:
    cells = " | ".join(f"{r[c]*100:5.1f}" for c in COLS)
    return f"| {label:<22} | {cells} |{extra}"


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    ap.add_argument("--out", type=Path, default=DEFAULT_OUT)
    ap.add_argument("--test-frac", type=float, default=0.15)
    ap.add_argument("--min-area-tokens", type=int, default=20000)
    ap.add_argument("--sample-frac", type=float, default=1.0,
                    help="evaluate KSR on this fraction of words (speed knob)")
    ap.add_argument("--margins", type=str, default="8",
                    help="comma list of auto-detect confidence margins to sweep "
                         "(log-ratio lead over runner-up), e.g. '1,2,4,8'")
    ap.add_argument("--no-breakdown", action="store_true",
                    help="skip the per-area breakdown (faster margin sweeps)")
    ap.add_argument("--max-cand", type=int, default=9)
    args = ap.parse_args()
    margins = [float(x) for x in args.margins.split(",") if x.strip()]

    if not args.corpus.exists():
        print(f"corpus not found: {args.corpus} (pass --corpus PATH)", file=sys.stderr)
        return 1

    data = load(args.corpus, args.test_frac)

    print("training global model…", file=sys.stderr)
    gmodel = Model.train(chain.from_iterable(d["train"] for d in data.values()),
                         PruneCfg.production())

    def tok_count(docs):
        return sum(len(t) for t in docs)

    areas = sorted(
        (a for a, d in data.items()
         if a != OTHER and tok_count(d["train"]) >= args.min_area_tokens),
        key=lambda a: -tok_count(data[a]["train"]),
    )
    print(f"area models: {len(areas)} (>= {args.min_area_tokens:,} train tokens)",
          file=sys.stderr)

    # Per-area models + blends, and the area classifier (built from raw train counts).
    blends: dict[str, BlendModel] = {}
    amodels: dict[str, Model] = {}
    area_counts: dict[str, Counter] = {}
    background: Counter = Counter()
    for d in data.values():
        for toks in d["train"]:
            background.update(toks)
    for area in areas:
        amodels[area] = Model.train(data[area]["train"], PruneCfg.light())
        blends[area] = BlendModel(amodels[area], gmodel)
        c: Counter = Counter()
        for toks in data[area]["train"]:
            c.update(toks)
        area_counts[area] = c
    classifier = AreaClassifier(area_counts, background)

    lines: list[str] = []
    lines.append("# ainuKey prediction benchmark\n")
    lines.append(
        f"Document-level split: test_frac={args.test_frac}, max_cand={args.max_cand}, "
        f"sample_frac={args.sample_frac}, margins={margins}. All numbers are "
        f"percentages. Headline = **KSR** (keystroke savings); `KSR_iv` = in-vocab only.\n"
    )

    # --- regime comparison: global vs oracle vs auto (margin sweep) ---
    print("evaluating regimes (global / oracle / auto sweep)…", file=sys.stderr)
    g_row, o_row, auto = eval_regimes(
        areas, data, gmodel, blends, classifier, margins, args.max_cand, args.sample_frac
    )
    d_oracle = (o_row["KSR"] - g_row["KSR"]) * 100
    lines.append("## Regimes — global vs oracle vs auto (trained-area test set)\n")
    lines.append(HEADER)
    lines.append(SEP)
    lines.append(fmt_row("global (baseline)", g_row))
    lines.append(fmt_row("oracle (true area)", o_row, f"  ΔKSR {d_oracle:+.1f}"))
    for m in margins:
        a_row, _ = auto[m]
        d_auto = (a_row["KSR"] - g_row["KSR"]) * 100
        lines.append(fmt_row(f"auto (margin {m:g})", a_row, f"  ΔKSR {d_auto:+.1f}"))
    lines.append("")

    # Margin sweep summary → recommend the margin that captures the most lift.
    lines.append("### Auto-detect margin sweep\n")
    lines.append("| margin | ΔKSR | capture | cls-acc | commit-rate |")
    lines.append("|--------|------|---------|---------|-------------|")
    best_m, best_capture = margins[0], -1.0
    for m in margins:
        a_row, st = auto[m]
        d_auto = (a_row["KSR"] - g_row["KSR"]) * 100
        capture = (d_auto / d_oracle * 100) if d_oracle > 0 else 0.0
        commit = (1 - st["cls_fallback_rate"]) * 100
        lines.append(f"| {m:g} | {d_auto:+.2f} | {capture:.0f}% | "
                     f"{st['cls_acc']*100:.1f}% | {commit:.0f}% |")
        if capture > best_capture:
            best_m, best_capture = m, capture
    a_best, st_best = auto[best_m]
    d_best = (a_best["KSR"] - g_row["KSR"]) * 100
    lines.append("")
    lines.append(
        f"- **Best: margin {best_m:g}** → ΔKSR {d_best:+.2f} pts, "
        f"**{best_capture:.0f}% of the oracle lift**, classifier "
        f"{st_best['cls_acc']*100:.1f}% accurate, commits on "
        f"{(1-st_best['cls_fallback_rate'])*100:.0f}% of positions."
    )
    lines.append("- Cold-start at best margin (accuracy by word position): " +
                 ", ".join(f"{name} **{acc*100:.0f}%**" for name, acc, _ in st_best["buckets"]))
    lines.append("")
    # values reused by the final verdict
    d_auto, capture, stats = d_best, best_capture, st_best

    # --- overall baseline over the WHOLE test set ---
    print("evaluating global over ALL test…", file=sys.stderr)
    all_test = list(chain.from_iterable(d["test"] for d in data.values()))
    overall = Metrics().eval_engine(gmodel, all_test, args.max_cand, args.sample_frac).row()
    lines.append("## Overall (whole-corpus test set, global model)\n")
    lines.append(HEADER)
    lines.append(SEP)
    lines.append(fmt_row("global", overall))
    lines.append("")

    # --- per-area breakdown: global vs area vs blend ---
    summary = []
    if not args.no_breakdown:
      lines.append("## Per area — global vs area-only vs area+global blend\n")
      for area in areas:
        test_docs = data[area]["test"]
        ntok = tok_count(test_docs)
        if ntok == 0:
            continue
        print(f"  area: {area} (test {ntok:,} tok)…", file=sys.stderr)
        g = Metrics().eval_engine(gmodel, test_docs, args.max_cand, args.sample_frac).row()
        a = Metrics().eval_engine(amodels[area], test_docs, args.max_cand, args.sample_frac).row()
        b = Metrics().eval_engine(blends[area], test_docs, args.max_cand, args.sample_frac).row()
        summary.append((area, g, a, b, ntok))
        d_ksr = (b["KSR"] - g["KSR"]) * 100
        d_h3 = (b["hit@3"] - g["hit@3"]) * 100
        lines.append(f"### {area}  ({ntok:,} test tokens)\n")
        lines.append(HEADER)
        lines.append(SEP)
        lines.append(fmt_row("global", g))
        lines.append(fmt_row("area-only", a))
        lines.append(fmt_row("blend (area+global)", b, f"  ΔKSR {d_ksr:+.1f}, Δhit@3 {d_h3:+.1f}"))
        lines.append("")

    if summary:
        tot = sum(n for *_, n in summary)
        w_ksr = sum((b["KSR"] - g["KSR"]) * n for _, g, _, b, n in summary) / tot * 100
        w_h3 = sum((b["hit@3"] - g["hit@3"]) * n for _, g, _, b, n in summary) / tot * 100
        lines.append("## Verdict\n")
        lines.append(f"- Oracle blend lift over global (token-weighted): **ΔKSR {w_ksr:+.2f}**, "
                     f"Δhit@3 {w_h3:+.2f} pts.")
        lines.append(f"- Auto-detect captures **{capture:.0f}%** of that lift with no user action "
                     f"(ΔKSR {d_auto:+.2f} pts), classifier {stats['cls_acc']*100:.0f}% accurate.")
        lines.append("")

    report = "\n".join(lines)
    print("\n" + report)
    args.out.write_text(report, encoding="utf-8")
    print(f"\nwrote {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
