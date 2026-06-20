#!/usr/bin/env python3
"""Keystroke-Savings-Rate for the trained neural model, using the SAME
simulation as tools/bench_ngrams.py so the number is directly comparable.

For each held-out word, we stream the LSTM over the real left context, then
simulate typing the word: at each prefix length the IME would show the top
completions among vocab words starting with what's typed, ranked by the model's
next-word distribution. The earliest prefix at which the true word appears in
the candidate list is where the user accepts it. KSR = letters saved / letters
typed. (Matches bench: max_cand slots with the typed prefix occupying slot 0, so
up to max_cand-1 real completions; accept cost = p typed + 1 selection.)

Run (needs the same env as training):
    uv run --with torch --with numpy --index https://download.pytorch.org/whl/cpu \
        tools/neural/eval_ksr.py [--run DIR] [--data DIR] [--sample-frac F]
"""
from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from pathlib import Path

import numpy as np
import torch

sys.path.insert(0, str(Path(__file__).resolve().parent))
from train import BOS, EOS, LSTMLM, PAD, UNK  # noqa: E402


def build_prefix_index(itos):
    """prefix -> np.array of vocab ids (real words only) starting with it."""
    idx = defaultdict(list)
    for i, w in enumerate(itos):
        if i < 4:  # skip <pad><unk><bos><eos>
            continue
        for p in range(1, len(w) + 1):
            idx[w[:p]].append(i)
    return {k: np.asarray(v, dtype=np.int64) for k, v in idx.items()}


@torch.no_grad()
def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--run", type=Path, default=Path("tools/neural/runs/small"))
    ap.add_argument("--data", type=Path, default=Path("data/neural"))
    ap.add_argument("--max-cand", type=int, default=9)
    ap.add_argument("--maxlen", type=int, default=64)
    ap.add_argument("--sample-frac", type=float, default=1.0)
    ap.add_argument("--batch", type=int, default=128)
    args = ap.parse_args()

    vocab = json.loads((args.data / "vocab.json").read_text(encoding="utf-8"))
    itos, stoi = vocab["itos"], vocab["stoi"]
    cfg = json.loads((args.run / "config.json").read_text(encoding="utf-8"))
    model = LSTMLM(cfg["vocab"], cfg["emb"], cfg["hid"], cfg["layers"], 0.0, cfg["tied"])
    model.load_state_dict(torch.load(args.run / "model.pt", map_location="cpu"))
    model.eval()
    pidx = build_prefix_index(itos)
    slots = args.max_cand - 1  # real completions (slot 0 is the typed prefix)

    # Surface sentences (for typing simulation) + their ids (for the model).
    sents = []
    for line in (args.data / "test.txt").read_text(encoding="utf-8").splitlines():
        toks = line.split()[: args.maxlen]
        if toks:
            sents.append(toks)

    chars_plain = chars_saved = 0
    cp_iv = cs_iv = 0
    seen = 0

    for b in range(0, len(sents), args.batch):
        chunk = sents[b : b + args.batch]
        # feed [BOS, w1..w_{n-1}] so logits[t] predicts w_{t+1} (t=0 -> w1).
        maxlen = max(len(s) for s in chunk)
        x = torch.full((len(chunk), maxlen), PAD, dtype=torch.long)
        for r, s in enumerate(chunk):
            ids = [BOS] + [stoi.get(t, UNK) for t in s[:-1]]
            x[r, : len(ids)] = torch.tensor(ids, dtype=torch.long)
        logits, _ = model(x)  # [B, maxlen, V]
        logits = logits.numpy()

        for r, s in enumerate(chunk):
            for i, tw in enumerate(s):  # predict word i from logits[r, i]
                L = len(tw)
                if L < 2:
                    continue
                seen += 1
                if (seen % 10000) >= int(args.sample_frac * 10000):
                    continue
                in_vocab = tw in stoi
                chars_plain += L
                if in_vocab:
                    cp_iv += L
                lg = logits[r, i]
                tid = stoi.get(tw, UNK)
                tlogit = lg[tid]
                best = L
                for p in range(1, L):
                    cand = pidx.get(tw[:p])
                    if cand is None or not in_vocab:
                        continue
                    # target in top-`slots` among prefix candidates?
                    if int((lg[cand] > tlogit).sum()) < slots:
                        best = p + 1
                        break
                saved = max(0, L - best)
                chars_saved += saved
                if in_vocab:
                    cs_iv += saved

    ksr = chars_saved / max(1, chars_plain)
    ksr_iv = cs_iv / max(1, cp_iv)
    out = {
        "KSR": round(ksr * 100, 1),
        "KSR_invocab": round(ksr_iv * 100, 1),
        "chars_plain": chars_plain,
        "max_cand": args.max_cand,
        "sample_frac": args.sample_frac,
    }
    print(json.dumps(out, indent=2))
    print("\n(compare: n-gram global KSR 31.7% whole-test; "
          "area-blend oracle 37.7% / auto 35.3% on trained-area subset)",
          file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
