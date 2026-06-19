# ainuKey prediction benchmark

Document-level split: test_frac=0.15, max_cand=9, sample_frac=1.0, margins=[0.5, 1.0, 2.0, 4.0, 8.0]. All numbers are percentages. Headline = **KSR** (keystroke savings); `KSR_iv` = in-vocab only.

## Regimes — global vs oracle vs auto (trained-area test set)

| engine / area          |   KSR | KSR_iv | hit@1 | hit@3 | hit@5 |   MRR |  fire |   OOV |
|------------------------|-------|-------|-------|-------|-------|-------|-------|-------|
| global (baseline)      |  32.7 |  40.5 |  25.3 |  37.7 |  43.3 |  32.8 |  93.4 |   8.5 |
| oracle (true area)     |  37.7 |  42.9 |  27.0 |  40.1 |  45.9 |  35.1 |  94.6 |   4.5 |  ΔKSR +5.0
| auto (margin 0.5)      |  36.1 |  42.2 |  26.1 |  39.0 |  44.7 |  34.1 |  94.4 |   5.3 |  ΔKSR +3.4
| auto (margin 1)        |  35.9 |  42.2 |  26.3 |  39.1 |  44.8 |  34.2 |  94.3 |   5.5 |  ΔKSR +3.2
| auto (margin 2)        |  35.7 |  42.1 |  26.3 |  39.1 |  44.7 |  34.1 |  94.1 |   5.7 |  ΔKSR +3.0
| auto (margin 4)        |  35.5 |  42.0 |  26.2 |  38.9 |  44.6 |  34.0 |  94.1 |   5.9 |  ΔKSR +2.8
| auto (margin 8)        |  35.3 |  41.9 |  26.0 |  38.8 |  44.5 |  33.8 |  94.0 |   6.2 |  ΔKSR +2.6

### Auto-detect margin sweep

| margin | ΔKSR | capture | cls-acc | commit-rate |
|--------|------|---------|---------|-------------|
| 0.5 | +3.42 | 68% | 84.3% | 65% |
| 1 | +3.24 | 65% | 91.3% | 55% |
| 2 | +3.04 | 61% | 97.4% | 47% |
| 4 | +2.82 | 56% | 99.6% | 41% |
| 8 | +2.58 | 51% | 100.0% | 36% |

- **Best: margin 0.5** → ΔKSR +3.42 pts, **68% of the oracle lift**, classifier 84.3% accurate, commits on 65% of positions.
- Cold-start at best margin (accuracy by word position): 1–5 **69%**, 6–20 **91%**, 21–50 **100%**, 51+ **100%**

## Overall (whole-corpus test set, global model)

| engine / area          |   KSR | KSR_iv | hit@1 | hit@3 | hit@5 |   MRR |  fire |   OOV |
|------------------------|-------|-------|-------|-------|-------|-------|-------|-------|
| global                 |  31.7 |  39.9 |  24.6 |  36.7 |  42.2 |  31.9 |  93.0 |   9.2 |

## Per area — global vs area-only vs area+global blend

### アイヌ語アーカイブ  (62,677 test tokens)

| engine / area          |   KSR | KSR_iv | hit@1 | hit@3 | hit@5 |   MRR |  fire |   OOV |
|------------------------|-------|-------|-------|-------|-------|-------|-------|-------|
| global                 |  31.4 |  40.0 |  25.1 |  38.9 |  45.1 |  33.4 |  92.1 |   7.0 |
| area-only              |  34.4 |  41.7 |  25.8 |  39.9 |  46.1 |  34.3 |  92.3 |   5.5 |
| blend (area+global)    |  34.4 |  41.4 |  26.3 |  40.5 |  46.9 |  35.1 |  93.1 |   5.1 |  ΔKSR +2.9, Δhit@3 +1.6

### アイヌ語訳新約聖書  (53,012 test tokens)

| engine / area          |   KSR | KSR_iv | hit@1 | hit@3 | hit@5 |   MRR |  fire |   OOV |
|------------------------|-------|-------|-------|-------|-------|-------|-------|-------|
| global                 |  37.3 |  42.3 |  28.2 |  40.2 |  45.5 |  35.4 |  96.6 |   6.7 |
| area-only              |  45.3 |  47.1 |  29.7 |  42.6 |  48.4 |  37.8 |  97.7 |   2.0 |
| blend (area+global)    |  44.0 |  45.7 |  29.7 |  42.4 |  48.1 |  37.7 |  97.9 |   2.0 |  ΔKSR +6.7, Δhit@3 +2.2

### 平取町アイヌ口承文芸  (12,747 test tokens)

| engine / area          |   KSR | KSR_iv | hit@1 | hit@3 | hit@5 |   MRR |  fire |   OOV |
|------------------------|-------|-------|-------|-------|-------|-------|-------|-------|
| global                 |  29.9 |  38.3 |  21.7 |  33.4 |  39.5 |  29.1 |  92.4 |   8.6 |
| area-only              |  33.2 |  40.4 |  21.7 |  33.9 |  39.7 |  29.2 |  89.7 |   7.4 |
| blend (area+global)    |  33.5 |  40.0 |  22.9 |  35.7 |  42.1 |  31.1 |  92.9 |   6.4 |  ΔKSR +3.6, Δhit@3 +2.3

### アイヌタイムズ  (15,634 test tokens)

| engine / area          |   KSR | KSR_iv | hit@1 | hit@3 | hit@5 |   MRR |  fire |   OOV |
|------------------------|-------|-------|-------|-------|-------|-------|-------|-------|
| global                 |  27.1 |  36.6 |  19.2 |  29.8 |  34.8 |  25.6 |  89.9 |  13.5 |
| area-only              |  33.2 |  41.4 |  21.9 |  32.7 |  37.2 |  28.5 |  89.0 |   9.9 |
| blend (area+global)    |  32.6 |  39.6 |  22.3 |  33.7 |  38.1 |  29.2 |  91.5 |   8.6 |  ΔKSR +5.5, Δhit@3 +3.9

### AA研アイヌ語資料  (4,414 test tokens)

| engine / area          |   KSR | KSR_iv | hit@1 | hit@3 | hit@5 |   MRR |  fire |   OOV |
|------------------------|-------|-------|-------|-------|-------|-------|-------|-------|
| global                 |  31.4 |  37.9 |  23.8 |  36.6 |  42.2 |  31.5 |  94.8 |   6.6 |
| area-only              |  34.0 |  40.2 |  23.0 |  35.0 |  40.8 |  30.3 |  91.4 |   6.0 |
| blend (area+global)    |  34.0 |  39.2 |  24.5 |  37.5 |  43.7 |  32.6 |  94.9 |   4.9 |  ΔKSR +2.5, Δhit@3 +0.8

### アイヌ語口承文芸コーパス  (4,054 test tokens)

| engine / area          |   KSR | KSR_iv | hit@1 | hit@3 | hit@5 |   MRR |  fire |   OOV |
|------------------------|-------|-------|-------|-------|-------|-------|-------|-------|
| global                 |  33.2 |  39.1 |  21.6 |  33.8 |  39.5 |  29.0 |  94.8 |   6.9 |
| area-only              |  36.1 |  41.4 |  20.6 |  33.6 |  39.0 |  28.3 |  89.9 |   6.5 |
| blend (area+global)    |  36.9 |  41.1 |  23.1 |  36.2 |  42.7 |  31.5 |  95.1 |   5.0 |  ΔKSR +3.7, Δhit@3 +2.4

### 浅井タケ昔話全集 I, II  (9,187 test tokens)

| engine / area          |   KSR | KSR_iv | hit@1 | hit@3 | hit@5 |   MRR |  fire |   OOV |
|------------------------|-------|-------|-------|-------|-------|-------|-------|-------|
| global                 |  32.2 |  45.1 |  29.9 |  41.3 |  45.8 |  36.6 |  88.9 |  18.5 |
| area-only              |  45.6 |  50.8 |  32.2 |  45.4 |  50.6 |  39.9 |  91.0 |   6.1 |
| blend (area+global)    |  43.0 |  47.8 |  32.2 |  45.4 |  49.9 |  39.9 |  91.5 |   5.8 |  ΔKSR +10.8, Δhit@3 +4.1

### アイヌ語ラジオ講座テキスト  (3,677 test tokens)

| engine / area          |   KSR | KSR_iv | hit@1 | hit@3 | hit@5 |   MRR |  fire |   OOV |
|------------------------|-------|-------|-------|-------|-------|-------|-------|-------|
| global                 |  25.2 |  35.1 |  15.3 |  26.2 |  30.3 |  21.8 |  90.4 |  15.5 |
| area-only              |  32.6 |  39.8 |  18.2 |  29.2 |  33.4 |  24.5 |  84.6 |   9.9 |
| blend (area+global)    |  31.4 |  37.1 |  21.2 |  32.3 |  37.3 |  28.0 |  91.9 |   7.9 |  ΔKSR +6.2, Δhit@3 +6.1

## Verdict

- Oracle blend lift over global (token-weighted): **ΔKSR +4.97**, Δhit@3 +2.32 pts.
- Auto-detect captures **68%** of that lift with no user action (ΔKSR +3.42 pts), classifier 84% accurate.
