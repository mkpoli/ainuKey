#!/usr/bin/env python3
"""Airtight KSR comparison: n-gram global vs n-gram area-oracle vs neural vs
neural+area fusion — all through ONE acceptance loop over the SAME test
positions, so the only thing that differs is how each engine ranks completions.

Removes the two caveats from the first neural KSR run:
  * same test set (whole held-out, area-labelled from the corpus), and
  * same simulator (identical 8-slot / accept-at-p+1 rule for every engine).

Engines per position (predict word w_i while typing it):
  * global  — n-gram candidate_list with the whole-corpus model (2-word context)
  * area    — n-gram candidate_list with the true area's blend (oracle area)
  * neural  — full-context LSTM logits, rank vocab words matching the prefix
  * fusion  — α·z(neural logit) + (1-α)·z(area signal), swept over a few α

Run (same env as training):
    uv run --with torch --with numpy --index https://download.pytorch.org/whl/cpu \
        tools/neural/compare_ksr.py [--sample-frac 0.4]
"""
from __future__ import annotations

import argparse
import json
import math
import sys
from collections import defaultdict
from pathlib import Path

import numpy as np
import torch

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))          # tools/  (ngram_lib, bench_ngrams)
sys.path.insert(0, str(Path(__file__).resolve().parent))  # tools/neural (train)

from bench_ngrams import OTHER, load  # noqa: E402
from ngram_lib import BlendModel, Model, PruneCfg, candidate_list  # noqa: E402
from train import BOS, LSTMLM, PAD, UNK  # noqa: E402

CORPUS = Path("../ainu-corpora/data.jsonl")
MAXCAND = 9
SLOTS = MAXCAND - 1


def neural_prefix_index(itos):
    idx = defaultdict(list)
    for i, w in enumerate(itos):
        if i < 4:
            continue
        for p in range(1, len(w) + 1):
            idx[w[:p]].append(i)
    return {k: np.asarray(v, np.int64) for k, v in idx.items()}


class KSR:
    def __init__(self):
        self.plain = 0
        self.saved = 0

    def add(self, L, accept_cost):
        self.plain += L
        self.saved += max(0, L - accept_cost)

    @property
    def rate(self):
        return 100.0 * self.saved / max(1, self.plain)


def accept_ngram(model, prev2, prev1, target, L):
    for p in range(1, L):
        cands = candidate_list(model, prev2, prev1, target[:p], MAXCAND)
        if target in cands[1:]:
            return p + 1
    return L


def accept_neural(lg, npidx, target, tid, L):
    tl = lg[tid]
    for p in range(1, L):
        cand = npidx.get(target[:p])
        if cand is None:
            continue
        if int((lg[cand] > tl).sum()) < SLOTS:
            return p + 1
    return L


def _z(a):
    a = a.astype(np.float64)
    s = a.std()
    return (a - a.mean()) / s if s > 1e-9 else a * 0.0


def accept_fusion(lg, npidx, itos, target, tid, L, area_pred, area_uni, alpha):
    for p in range(1, L):
        cand = npidx.get(target[:p])
        if cand is None:
            continue
        if tid not in cand:
            continue
        zneu = _z(lg[cand])
        sig = np.array([math.log(area_uni.get(itos[i], 1)) + 3.0 * area_pred.get(itos[i], 0.0)
                        for i in cand])
        fused = alpha * zneu + (1 - alpha) * _z(sig)
        tpos = int(np.where(cand == tid)[0][0])
        if int((fused > fused[tpos]).sum()) < SLOTS:
            return p + 1
    return L


@torch.no_grad()
def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--run", type=Path, default=Path("tools/neural/runs/small"))
    ap.add_argument("--data", type=Path, default=Path("data/neural"))
    ap.add_argument("--corpus", type=Path, default=CORPUS)
    ap.add_argument("--test-frac", type=float, default=0.15)
    ap.add_argument("--min-area-tokens", type=int, default=20000)
    ap.add_argument("--sample-frac", type=float, default=0.4)
    ap.add_argument("--alphas", type=str, default="0.5,0.7,0.9")
    args = ap.parse_args()
    alphas = [float(a) for a in args.alphas.split(",")]

    # --- neural model ---
    vocab = json.loads((args.data / "vocab.json").read_text())
    itos, stoi = vocab["itos"], vocab["stoi"]
    cfg = json.loads((args.run / "config.json").read_text())
    model = LSTMLM(cfg["vocab"], cfg["emb"], cfg["hid"], cfg["layers"], 0.0, cfg["tied"])
    model.load_state_dict(torch.load(args.run / "model.pt", map_location="cpu"))
    model.eval()
    npidx = neural_prefix_index(itos)

    # --- n-gram global + per-area blends (same as bench) ---
    print("loading corpus + training n-gram models…", file=sys.stderr)
    data = load(args.corpus, args.test_frac)
    gmodel = Model.train(
        (t for d in data.values() for t in d["train"]), PruneCfg.production())
    areas = {a for a, d in data.items()
             if a != OTHER and sum(len(t) for t in d["train"]) >= args.min_area_tokens}
    blends, area_uni = {}, {}
    for a in areas:
        am = Model.train(data[a]["train"], PruneCfg.light())
        blends[a] = BlendModel(am, gmodel)
        area_uni[a] = {w: c for w, c in blends[a].unigrams}
    print(f"areas={len(areas)}", file=sys.stderr)

    engines = ["global", "area", "neural"] + [f"fusion@{a:g}" for a in alphas]
    ksr = {e: KSR() for e in engines}
    seen = 0

    for area, d in data.items():
        blend = blends.get(area)
        ap_area = blend  # area-oracle engine (or global if untrained area)
        for words in d["test"]:
            words = words[:64]
            if len(words) < 2:
                continue
            # neural forward over [BOS, w1..w_{n-1}] -> logits[i] predicts words[i]
            ids = [BOS] + [stoi.get(w, UNK) for w in words[:-1]]
            x = torch.tensor([ids], dtype=torch.long)
            lg = model(x)[0][0].numpy()  # [len(words), V]
            for i, tw in enumerate(words):
                L = len(tw)
                if L < 2:
                    continue
                seen += 1
                if (seen % 10000) >= int(args.sample_frac * 10000):
                    continue
                prev1 = words[i - 1] if i >= 1 else None
                prev2 = words[i - 2] if i >= 2 else None
                tid = stoi.get(tw, UNK)
                in_vocab = tw in stoi
                lgi = lg[i]
                ksr["global"].add(L, accept_ngram(gmodel, prev2, prev1, tw, L))
                amodel = ap_area if ap_area is not None else gmodel
                ksr["area"].add(L, accept_ngram(amodel, prev2, prev1, tw, L))
                ksr["neural"].add(L, accept_neural(lgi, npidx, tw, tid, L) if in_vocab else L)
                if in_vocab and blend is not None:
                    apred = blend.predict_scores(prev2, prev1)
                    auni = area_uni[area]
                    for a in alphas:
                        ksr[f"fusion@{a:g}"].add(
                            L, accept_fusion(lgi, npidx, itos, tw, tid, L, apred, auni, a))
                else:
                    # no area model (or OOV) → fusion falls back to neural's result
                    nc = accept_neural(lgi, npidx, tw, tid, L) if in_vocab else L
                    for a in alphas:
                        ksr[f"fusion@{a:g}"].add(L, nc)

    print(f"\nsampled positions: {ksr['global'].plain and seen} "
          f"(chars_plain={ksr['global'].plain})", file=sys.stderr)
    print("\n=== KSR (same positions, same acceptance rule) ===")
    base = ksr["global"].rate
    for e in engines:
        r = ksr[e].rate
        print(f"  {e:<12} {r:5.1f}   (Δ vs global {r - base:+.1f})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
