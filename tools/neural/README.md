# Neural next-word model — experiment

A small word-level **LSTM** next-word model for the IME, trained from scratch on
the Ainu corpus, compared head-to-head against the rule-based / n-gram engine in
the *same* benchmark. Goal: decide whether a neural predictor is worth shipping.

Everything reuses the IME's exact tokenizer (`../ngram_lib.py`) and the **same
document-level train/test split** as `../bench_ngrams.py`, so neural and n-gram
are measured on identical held-out data with identical metrics.

## Scripts

| file | what | needs |
|---|---|---|
| `prep_data.py` | tokenize corpus → vocab + train/test sentences (`data/neural/`) | stdlib only |
| `train.py` | train the LSTM, report ppl + hit@1/3/5/9 + MRR, export ONNX | torch |
| `eval_ksr.py` | keystroke-savings of the trained model (streaming sim) | torch + numpy |
| `compare_ksr.py` | **airtight** KSR: global / area / neural / fusion, one harness | torch + numpy |

Env-driven (`NM_*`, see `train.py`) so the same script runs locally or on a
rented GPU (`pytorch/pytorch` image needs no extra installs — trained from
scratch). At this size it trains on **CPU in ~12 min**; no GPU required.

```bash
uv run tools/neural/prep_data.py
NM_DEVICE=cpu NM_EPOCHS=20 NM_OUT=tools/neural/runs/small \
  uv run --with torch --with numpy --index https://download.pytorch.org/whl/cpu \
  tools/neural/train.py
uv run --with torch --with numpy --index https://download.pytorch.org/whl/cpu \
  tools/neural/compare_ksr.py --sample-frac 0.5
```

## Result (corpus snapshot 2026-06-20)

Model: word-level LSTM, emb 128 / hid 256 / 1 layer, vocab 10k — **4.2M params,
~4 MB int8**. Overfits by epoch ~7 (val ppl floor), confirming the **1.09M-token
corpus is the bottleneck**, not capacity.

Keystroke-Savings Rate, identical positions + acceptance rule for every engine:

| engine | KSR | Δ vs global |
|---|---|---|
| n-gram global (shipped) | 31.3 | — |
| n-gram area-oracle | 35.6 | +4.3 |
| **neural LSTM** | **38.3** | **+7.0** |
| neural+area fusion | ≤ 38.3 | converges to pure neural |

**Conclusions**

- Neural beats the area-specialized n-gram head-to-head (**+2.7 KSR**) and the
  global n-gram by **+7.0**. Its edge is *completion while typing* — full-sentence
  context ranks completions far earlier than a 2-word n-gram (hit@3 understates
  it: only +1.3 there, but +7 on KSR, the metric users feel).
- **Fusion adds nothing**: the area signal is fully subsumed by the neural model's
  context. The best engine is also the simplest — a *single global LSTM*, no area
  tables, classifier, or fusion.

→ Worth shipping. Deployment = ONNX export + a Rust `ort` runtime in the DLL,
streaming the LSTM hidden state per keystroke, behind a config flag with n-gram
fallback. (Model artifacts and `data/neural/` are git-ignored — corpus-derived.)
