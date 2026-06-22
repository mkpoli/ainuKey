#!/usr/bin/env python3
"""Export the trained LSTM to a compact `lstm.bin` the Rust DLL can `include_bytes!`
and run with a hand-rolled forward pass (no ONNX runtime).

Big matrices (embedding, the two LSTM weight matrices, the output projection —
~4.2M of the 4.25M params) are per-tensor symmetric **int8** with an f32 scale;
the small bias vectors stay f32. That lands ~4 MB on disk while keeping the
compute in f32 after a cheap dequant at load.

To prove the quantization doesn't cost accuracy, `--emit-checkpoint DIR` also
writes a *dequantized* model.pt (= the exact weights the Rust side will use), so
`eval_ksr.py --run DIR` measures the as-shipped KSR.

Binary layout (little-endian):
  magic "AKLS" | u32 version=1
  u32 V | u32 E | u32 H | u8 layers(=1) | u8 quant(0=f32,1=int8 big matrices)
  vocab: V × { u8 len, utf8 bytes }                 (itos; ids 0..V)
  big matrices, in order  emb[V,E]  Wih[4H,E]  Whh[4H,H]  projW[V,H] :
     int8 → f32 scale, then rows*cols × i8     |  f32 → rows*cols × f32
  bias vectors, in order  bih[4H]  bhh[4H]  projB[V] :  rows × f32

Run:
    uv run --with torch --with numpy --index https://download.pytorch.org/whl/cpu \
        tools/neural/export_weights.py [--quant int8] [--emit-checkpoint DIR]
"""
from __future__ import annotations

import argparse
import json
import struct
import sys
from pathlib import Path

import numpy as np
import torch

MAGIC = b"AKLS"
VERSION = 1
# Big matrices to quantize, in the binary's fixed order.
BIG = ["emb.weight", "lstm.weight_ih_l0", "lstm.weight_hh_l0", "proj.weight"]
BIAS = ["lstm.bias_ih_l0", "lstm.bias_hh_l0", "proj.bias"]


def quant_int8(w: np.ndarray):
    """Per-tensor symmetric int8: q ≈ w / scale, scale = max|w| / 127."""
    scale = float(np.max(np.abs(w))) / 127.0
    if scale == 0.0:
        scale = 1.0
    q = np.clip(np.round(w / scale), -127, 127).astype(np.int8)
    return q, scale


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--run", type=Path, default=Path("tools/neural/runs/small"))
    ap.add_argument("--data", type=Path, default=Path("data/neural"))
    ap.add_argument("--out", type=Path, default=Path("data/neural/lstm.bin"))
    ap.add_argument("--quant", choices=["int8", "f32"], default="int8")
    ap.add_argument("--emit-checkpoint", type=Path, default=None,
                    help="also write a dequantized model.pt here for KSR validation")
    ap.add_argument("--emit-parity", type=Path, default=None,
                    help="write a JSON parity fixture (explicit-math forward on the "
                         "dequantized weights) for the Rust module's test")
    args = ap.parse_args()

    cfg = json.loads((args.run / "config.json").read_text())
    assert cfg["layers"] == 1, "binary format v1 assumes a single LSTM layer"
    V, E, H = cfg["vocab"], cfg["emb"], cfg["hid"]
    sd = torch.load(args.run / "model.pt", map_location="cpu")
    w = {k: v.detach().cpu().numpy().astype(np.float32) for k, v in sd.items()}
    if cfg.get("tied"):
        w["proj.weight"] = w["emb.weight"]
    itos = json.loads((args.data / "vocab.json").read_text())["itos"]
    assert len(itos) == V, f"vocab mismatch {len(itos)} != {V}"

    quant = 1 if args.quant == "int8" else 0
    buf = bytearray()
    buf += MAGIC + struct.pack("<I", VERSION)
    buf += struct.pack("<IIIBB", V, E, H, 1, quant)
    for tok in itos:
        b = tok.encode("utf-8")
        assert len(b) < 256
        buf += struct.pack("<B", len(b)) + b

    dq = {}  # dequantized big matrices (for the checkpoint)
    max_err = 0.0
    for k in BIG:
        m = w[k]
        if quant:
            q, scale = quant_int8(m)
            buf += struct.pack("<f", scale)
            buf += q.tobytes()  # row-major int8
            deq = q.astype(np.float32) * scale
            dq[k] = deq
            max_err = max(max_err, float(np.max(np.abs(deq - m))))
        else:
            buf += m.astype("<f4").tobytes()
            dq[k] = m
    for k in BIAS:
        buf += w[k].astype("<f4").tobytes()

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_bytes(buf)
    mb = len(buf) / 1e6
    print(f"wrote {args.out} ({mb:.2f} MB, quant={args.quant}, "
          f"max dequant error={max_err:.5f})", file=sys.stderr)

    if args.emit_checkpoint:
        out = args.emit_checkpoint
        out.mkdir(parents=True, exist_ok=True)
        new_sd = {}
        for k, v in sd.items():
            new_sd[k] = torch.from_numpy(dq[k]) if k in dq else v
        torch.save(new_sd, out / "model.pt")
        (out / "config.json").write_text(json.dumps(cfg, indent=2))
        print(f"wrote dequantized checkpoint to {out} (run eval_ksr.py --run {out})",
              file=sys.stderr)

    if args.emit_parity:
        # Explicit single-layer LSTM forward (numpy) on the *exported* weights —
        # the same arithmetic the Rust module implements — so the Rust test can
        # assert byte-for-byte-ish parity. Gate order is PyTorch's i,f,g,o.
        emb = dq["emb.weight"]
        wih, whh = dq["lstm.weight_ih_l0"], dq["lstm.weight_hh_l0"]
        bih, bhh = w["lstm.bias_ih_l0"], w["lstm.bias_hh_l0"]
        projw, projb = dq["proj.weight"], w["proj.bias"]

        def sig(x):
            return 1.0 / (1.0 + np.exp(-x))

        def forward(ids):
            h = np.zeros(H, np.float32)
            c = np.zeros(H, np.float32)
            for t in ids:
                g = wih @ emb[t] + bih + whh @ h + bhh
                i_, f_ = sig(g[:H]), sig(g[H : 2 * H])
                gg, o_ = np.tanh(g[2 * H : 3 * H]), sig(g[3 * H :])
                c = f_ * c + i_ * gg
                h = o_ * np.tanh(c)
            logits = projw @ h + projb
            return h, logits

        cases = []
        for ids in ([2], [2, 5, 9, 13], [2, 100, 7, 42, 8]):
            h, logits = forward(ids)
            top = np.argsort(-logits)[:10]
            cases.append({
                "ids": ids,
                "h": [round(float(x), 6) for x in h],
                "top": [[int(t), round(float(logits[t]), 5)] for t in top],
            })
        args.emit_parity.parent.mkdir(parents=True, exist_ok=True)
        args.emit_parity.write_text(json.dumps({"hid": H, "cases": cases}))
        print(f"wrote parity fixture to {args.emit_parity}", file=sys.stderr)

    print(json.dumps({"bytes": len(buf), "mb": round(mb, 2),
                      "quant": args.quant, "max_dequant_error": round(max_err, 5)}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
