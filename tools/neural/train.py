#!/usr/bin/env python3
"""Train a small word-level LSTM next-word model for the IME and report the
SAME metrics the n-gram benchmark uses, so we can decide go/no-go.

Trained from scratch (no pretrained download) → the vast.ai pytorch image needs
only torch, no extra installs. Env-driven (runbook §8) so the same script runs
on any box:

  NM_DATA   data dir from prep_data.py        (default ./data/neural)
  NM_OUT    output dir                          (default ./runs/lstm)
  NM_EMB    embedding dim                        (128)
  NM_HID    LSTM hidden dim                      (256)
  NM_LAYERS LSTM layers                          (1)
  NM_DROPOUT dropout                             (0.2)
  NM_EPOCHS epochs                               (8)
  NM_BS     batch size (sentences)               (128)
  NM_LR     learning rate                        (2e-3)
  NM_TIED   tie input/output embeddings (1/0)    (0; needs EMB==HID)
  NM_DEVICE cuda|cpu                             (auto)

Metrics on the held-out split (predicting token i from BOS+prefix, i>=1, exactly
the n-gram benchmark's next-word task): perplexity, hit@1/3/5/9, MRR. A target
that maps to <unk> counts as a miss (mirrors the n-gram OOV handling), so the
numbers are directly comparable to bench_ngrams' global hit@k.

Exports an ONNX single-step recurrence (token_id, h, c) -> (logits, h, c) for
streaming CPU inference in the Rust DLL.
"""
from __future__ import annotations

import json
import math
import os
import sys
from pathlib import Path

import torch
import torch.nn as nn

PAD, UNK, BOS, EOS = 0, 1, 2, 3
KS = (1, 3, 5, 9)


def env(name, default, cast=str):
    v = os.environ.get(name)
    return cast(v) if v is not None and v != "" else default


def load_data(data_dir: Path, stoi: dict, maxlen: int):
    def encode(path):
        seqs = []
        for line in path.read_text(encoding="utf-8").splitlines():
            toks = line.split()[:maxlen]  # cap to bound batch memory
            if not toks:
                continue
            ids = [BOS] + [stoi.get(t, UNK) for t in toks] + [EOS]
            seqs.append(ids)
        return seqs

    return encode(data_dir / "train.txt"), encode(data_dir / "test.txt")


def batches(seqs, bs, device, shuffle):
    order = list(range(len(seqs)))
    if shuffle:
        g = torch.Generator().manual_seed(1234)
        order = torch.randperm(len(seqs), generator=g).tolist()
    for i in range(0, len(order), bs):
        chunk = [seqs[j] for j in order[i : i + bs]]
        maxlen = max(len(s) for s in chunk)
        x = torch.full((len(chunk), maxlen), PAD, dtype=torch.long)
        for r, s in enumerate(chunk):
            x[r, : len(s)] = torch.tensor(s, dtype=torch.long)
        yield x.to(device)


class LSTMLM(nn.Module):
    def __init__(self, vocab, emb, hid, layers, dropout, tied):
        super().__init__()
        self.emb = nn.Embedding(vocab, emb, padding_idx=PAD)
        self.lstm = nn.LSTM(emb, hid, layers, batch_first=True,
                            dropout=dropout if layers > 1 else 0.0)
        self.drop = nn.Dropout(dropout)
        self.proj = nn.Linear(hid, vocab)
        if tied:
            assert emb == hid, "weight tying needs NM_EMB == NM_HID"
            self.proj.weight = self.emb.weight

    def forward(self, x, state=None):
        e = self.drop(self.emb(x))
        out, state = self.lstm(e, state)
        return self.proj(self.drop(out)), state


@torch.no_grad()
def evaluate(model, seqs, bs, device):
    model.eval()
    nll, ntok = 0.0, 0
    hits = {k: 0 for k in KS}
    rr = 0.0
    positions = 0
    ce = nn.CrossEntropyLoss(ignore_index=PAD, reduction="sum")
    for x in batches(seqs, bs, device, shuffle=False):
        logits, _ = model(x[:, :-1])
        tgt = x[:, 1:]
        # perplexity over all real (non-pad) targets
        flat = logits.reshape(-1, logits.size(-1))
        nll += ce(flat, tgt.reshape(-1)).item()
        ntok += (tgt != PAD).sum().item()
        # hit@k / MRR over positions i>=1 (context has >=1 real word). Position 0
        # predicts the first word from BOS alone — excluded to match the n-gram
        # task (which needs prev1). A target of <unk> is always a miss.
        sub = logits[:, 1:, :]  # drop the BOS->first-word step
        st = tgt[:, 1:]
        # rank of the true target among all logits
        true_logit = sub.gather(-1, st.unsqueeze(-1)).squeeze(-1)
        rank = (sub > true_logit.unsqueeze(-1)).sum(-1) + 1  # 1-based
        is_unk = st == UNK
        # Count exactly the n-gram task: predict each REAL word w2..wn from >=1
        # real word of context. Exclude PAD and the EOS target (n-gram has no EOS);
        # <unk> targets stay in the denominator as guaranteed misses.
        countable = (st != PAD) & (st != EOS)
        positions += countable.sum().item()
        hitmask = countable & ~is_unk
        for k in KS:
            hits[k] += ((rank <= k) & hitmask).sum().item()
        rr += (hitmask.float() / rank.float()).sum().item()
    ppl = math.exp(nll / max(1, ntok))
    p = max(1, positions)
    return {
        "ppl": round(ppl, 2),
        "hit@1": round(hits[1] / p, 4),
        "hit@3": round(hits[3] / p, 4),
        "hit@5": round(hits[5] / p, 4),
        "hit@9": round(hits[9] / p, 4),
        "MRR": round(rr / p, 4),
        "positions": positions,
    }


def export_onnx(model, out: Path, device):
    """Single-step recurrence for streaming CPU inference in Rust (ort)."""
    model.eval()
    layers = model.lstm.num_layers
    hid = model.lstm.hidden_size

    class Step(nn.Module):
        def __init__(self, m):
            super().__init__()
            self.m = m

        def forward(self, token, h, c):
            logits, (h2, c2) = self.m(token, (h, c))
            return logits[:, -1, :], h2, c2

    step = Step(model).to(device)
    token = torch.tensor([[BOS]], dtype=torch.long, device=device)
    h = torch.zeros(layers, 1, hid, device=device)
    c = torch.zeros(layers, 1, hid, device=device)
    torch.onnx.export(
        step, (token, h, c), str(out),
        input_names=["token", "h_in", "c_in"],
        output_names=["logits", "h_out", "c_out"],
        dynamic_axes={"token": {0: "batch"}, "h_in": {1: "batch"},
                      "c_in": {1: "batch"}},
        opset_version=17,
    )


def main() -> int:
    data_dir = Path(env("NM_DATA", "./data/neural"))
    out = Path(env("NM_OUT", "./runs/lstm"))
    emb = env("NM_EMB", 128, int)
    hid = env("NM_HID", 256, int)
    layers = env("NM_LAYERS", 1, int)
    dropout = env("NM_DROPOUT", 0.2, float)
    epochs = env("NM_EPOCHS", 8, int)
    bs = env("NM_BS", 128, int)
    lr = env("NM_LR", 2e-3, float)
    tied = bool(int(env("NM_TIED", 0, int)))
    maxlen = env("NM_MAXLEN", 64, int)
    device = env("NM_DEVICE", "cuda" if torch.cuda.is_available() else "cpu")

    out.mkdir(parents=True, exist_ok=True)
    vocab = json.loads((data_dir / "vocab.json").read_text(encoding="utf-8"))
    stoi, itos = vocab["stoi"], vocab["itos"]
    V = len(itos)
    train_seqs, test_seqs = load_data(data_dir, stoi, maxlen)
    print(f"vocab={V} train_sents={len(train_seqs)} test_sents={len(test_seqs)} "
          f"device={device}", file=sys.stderr)

    model = LSTMLM(V, emb, hid, layers, dropout, tied).to(device)
    nparams = sum(p.numel() for p in model.parameters())
    fp32_mb = nparams * 4 / 1e6
    print(f"params={nparams:,} (~{fp32_mb:.1f} MB fp32, ~{fp32_mb/4:.1f} MB int8)",
          file=sys.stderr)

    opt = torch.optim.Adam(model.parameters(), lr=lr)
    ce = nn.CrossEntropyLoss(ignore_index=PAD)
    best = {"hit@3": -1.0}
    history = []
    for ep in range(1, epochs + 1):
        model.train()
        total, steps = 0.0, 0
        for x in batches(train_seqs, bs, device, shuffle=True):
            logits, _ = model(x[:, :-1])
            loss = ce(logits.reshape(-1, V), x[:, 1:].reshape(-1))
            opt.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            opt.step()
            total += loss.item()
            steps += 1
        metrics = evaluate(model, test_seqs, bs, device)
        metrics["epoch"] = ep
        metrics["train_loss"] = round(total / max(1, steps), 4)
        history.append(metrics)
        print(json.dumps(metrics), flush=True)
        if metrics["hit@3"] > best["hit@3"]:
            best = metrics
            torch.save(model.state_dict(), out / "model.pt")

    # Reload best, export ONNX + write a results summary.
    model.load_state_dict(torch.load(out / "model.pt", map_location=device))
    config = {"vocab": V, "emb": emb, "hid": hid, "layers": layers,
              "tied": tied, "params": nparams, "fp32_mb": round(fp32_mb, 1)}
    (out / "config.json").write_text(json.dumps(config, indent=2))
    try:
        export_onnx(model, out / "model.onnx", device)
        onnx_mb = (out / "model.onnx").stat().st_size / 1e6
        print(f"exported model.onnx (~{onnx_mb:.1f} MB)", file=sys.stderr)
    except Exception as e:  # export must not fail the metrics run
        print(f"ONNX export failed (non-fatal): {e}", file=sys.stderr)

    summary = {"config": config, "best": best, "history": history}
    (out / "results.json").write_text(json.dumps(summary, indent=2))
    print("\n=== BEST ===")
    print(json.dumps(best, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
