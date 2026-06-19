#!/usr/bin/env python3
"""Shared n-gram tokenization + model used by both the table builder
(`build_ngrams.py`) and the benchmark (`bench_ngrams.py`).

Keeping these in one place guarantees the benchmark measures the *same*
tokenization and the *same* ranking the shipped IME uses, so the numbers are
trustworthy. The model methods deliberately mirror the Rust engine:

  * `Model.predict_scores`  ↔  `src/suggest.rs::Suggestions::predict_scores`
    (stupid-backoff blend, bigram weight ALPHA = 0.4, each context layer
    normalized to its own top count).
  * `candidate_list`        ↔  `src/candidates.rs::CandidateList::build`
    (prefix completions of the current word, context re-ranked, deduped).
  * `Model.complete`        ↔  `src/suggest.rs::Suggestions::complete`.

All canonical words are lowercase Latin (the IME's internal form).
"""
from __future__ import annotations

import json
import re
from collections import Counter
from dataclasses import dataclass, field
from itertools import pairwise
from pathlib import Path
from typing import Iterable, Iterator, Protocol

# --- tokenization (the single source of truth) ------------------------------

MAX_WORD_BYTES = 40
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


def iter_records(corpus: Path) -> Iterator[dict]:
    """Yield each JSON record from a `.jsonl` corpus, skipping bad lines."""
    with corpus.open(encoding="utf-8") as f:
        for line in f:
            try:
                yield json.loads(line)
            except json.JSONDecodeError:
                continue


# --- pruning knobs (mirrors build_ngrams.py serialize()) --------------------


@dataclass(frozen=True)
class PruneCfg:
    """Pruning thresholds applied before a model is queried, so the benchmark
    measures a table the size the IME would actually ship."""

    max_unigrams: int = 4000
    min_context_count: int = 3  # drop rare bigram contexts
    top_k_next: int = 8  # next-words kept per bigram context
    min_next_count: int = 2  # drop rare bigram continuations
    min_tri_context: int = 4  # drop rare trigram contexts
    top_k_tri: int = 6  # next-words kept per trigram context
    min_tri_next: int = 2  # drop rare trigram continuations

    @staticmethod
    def production() -> "PruneCfg":
        """The exact knobs build_ngrams.py ships with."""
        return PruneCfg()

    @staticmethod
    def light() -> "PruneCfg":
        """Looser thresholds for a small (area) corpus, which has lower counts
        and would be gutted by the production thresholds."""
        return PruneCfg(
            max_unigrams=4000,
            min_context_count=2,
            top_k_next=8,
            min_next_count=1,
            min_tri_context=2,
            top_k_tri=6,
            min_tri_next=1,
        )

    @staticmethod
    def none() -> "PruneCfg":
        return PruneCfg(
            max_unigrams=10**9,
            min_context_count=1,
            top_k_next=10**9,
            min_next_count=1,
            min_tri_context=1,
            top_k_tri=10**9,
            min_tri_next=1,
        )


ALPHA = 0.4  # bigram weight in the stupid-backoff blend (matches suggest.rs)


class Engine(Protocol):
    """The surface `candidate_list` needs; both Model and BlendModel satisfy it."""

    def complete(self, prefix: str, n: int) -> list[tuple[str, int]]: ...
    def predict_scores(self, prev2: str | None, prev1: str) -> dict[str, float]: ...


# --- the n-gram model -------------------------------------------------------


@dataclass
class Model:
    """A trained, optionally-pruned n-gram model. Built once, then queried."""

    unigrams: list[tuple[str, int]]  # descending by count
    bigrams: dict[str, list[tuple[str, int]]]  # prev -> top nexts (desc)
    trigrams: dict[tuple[str, str], list[tuple[str, int]]]  # (p2,p1) -> top nexts
    _vocab: set[str] = field(default_factory=set, repr=False)
    _prefix: dict[str, list[tuple[str, int]]] = field(default_factory=dict, repr=False)

    # -- training --

    @classmethod
    def train(cls, docs: Iterable[list[str]], prune: PruneCfg | None = None) -> "Model":
        """Train from an iterable of token-lists (one per record/sentence)."""
        uni: Counter = Counter()
        bi: dict[str, Counter] = {}
        tri: dict[tuple[str, str], Counter] = {}
        for toks in docs:
            uni.update(toks)
            for a, b in pairwise(toks):
                bi.setdefault(a, Counter())[b] += 1
            for a, b, c in zip(toks, toks[1:], toks[2:]):
                tri.setdefault((a, b), Counter())[c] += 1
        return cls._from_counts(uni, bi, tri, prune or PruneCfg.production())

    @classmethod
    def _from_counts(cls, uni, bi, tri, p: PruneCfg) -> "Model":
        top_uni = uni.most_common(p.max_unigrams)

        def prune_ctx(ctx_counts, min_ctx, top_k, min_next):
            out = {}
            for ctx, cnt in ctx_counts.items():
                if sum(cnt.values()) < min_ctx:
                    continue
                top = [(w, c) for w, c in cnt.most_common(top_k) if c >= min_next]
                if top:
                    out[ctx] = top
            return out

        bigrams = prune_ctx(bi, p.min_context_count, p.top_k_next, p.min_next_count)
        trigrams = prune_ctx(tri, p.min_tri_context, p.top_k_tri, p.min_tri_next)
        m = cls(top_uni, bigrams, trigrams)
        m._index()
        return m

    def _index(self) -> None:
        """Build the prefix→completions index and the vocab set."""
        self._vocab = {w for w, _ in self.unigrams}
        idx: dict[str, list[tuple[str, int]]] = {}
        # `unigrams` is already frequency-sorted, so appending preserves order.
        for w, c in self.unigrams:
            for i in range(1, len(w) + 1):
                idx.setdefault(w[:i], []).append((w, c))
        self._prefix = idx

    # -- queries (mirror the Rust engine) --

    def complete(self, prefix: str, n: int) -> list[tuple[str, int]]:
        if not prefix:
            return []
        return self._prefix.get(prefix, [])[:n]

    def next_words(self, prev: str) -> list[tuple[str, int]]:
        return self.bigrams.get(prev, [])

    def predict_scores(self, prev2: str | None, prev1: str) -> dict[str, float]:
        """Blended next-word scores; see suggest.rs::predict_scores."""
        scores: dict[str, float] = {}

        def add(entries: list[tuple[str, int]], weight: float) -> None:
            if not entries:
                return
            mx = max((c for _, c in entries), default=0) or 1
            for w, c in entries:
                scores[w] = scores.get(w, 0.0) + weight * (c / mx)

        add(self.next_words(prev1), ALPHA)
        if prev2 is not None:
            add(self.trigrams.get((prev2, prev1), []), 1.0)
        return scores

    def default_words(self, n: int) -> list[tuple[str, int]]:
        return self.unigrams[:n]

    def __contains__(self, word: str) -> bool:
        return word in self._vocab


class BlendModel:
    """An area model boosted on top of a global model: in-domain words and
    collocations rank higher, but global coverage is retained for everything the
    small area never saw. This models a deployment where the user's writing
    domain ("area") is known and selected."""

    def __init__(self, area: Model, glob: Model, uni_boost: float = 6.0,
                 pred_weight: float = 1.5):
        self.area = area
        self.glob = glob
        self.pred_weight = pred_weight
        combined: Counter = Counter()
        for w, c in glob.unigrams:
            combined[w] += c
        for w, c in area.unigrams:
            combined[w] += int(uni_boost * c)
        self.unigrams = sorted(combined.items(), key=lambda kv: (-kv[1], kv[0]))
        self._vocab = set(combined)
        idx: dict[str, list[tuple[str, int]]] = {}
        for w, c in self.unigrams:
            for i in range(1, len(w) + 1):
                idx.setdefault(w[:i], []).append((w, c))
        self._prefix = idx

    def complete(self, prefix: str, n: int) -> list[tuple[str, int]]:
        if not prefix:
            return []
        return self._prefix.get(prefix, [])[:n]

    def predict_scores(self, prev2: str | None, prev1: str) -> dict[str, float]:
        g = self.glob.predict_scores(prev2, prev1)
        a = self.area.predict_scores(prev2, prev1)
        out = dict(g)
        for w, s in a.items():
            out[w] = out.get(w, 0.0) + self.pred_weight * s
        return out

    def __contains__(self, word: str) -> bool:
        return word in self._vocab


import math


class AreaClassifier:
    """Online "which writing domain is this?" classifier — cheap enough to run
    per word in the Rust hot path later. For each area it holds a smoothed
    unigram log-likelihood *ratio against the background* (the whole corpus), so
    words common everywhere contribute ~0 and only domain-distinctive words move
    the score. A document's running score is the sum of those ratios over the
    words typed so far; the predicted area is the arg-max (or `None` until the
    lead over the runner-up clears `margin`, so cold-start stays on global)."""

    def __init__(self, area_counts: dict[str, Counter], background: Counter,
                 k: float = 0.5):
        self.areas = list(area_counts)
        vocab = len(background)
        bg_total = sum(background.values()) + k * vocab
        # Background log-prob is kept SEPARATE from the area tables: a word can be
        # unseen in an area yet common in the corpus (the discriminative case), so
        # its background term must always count, even when the area term backs off
        # to the unseen floor.
        self._bg_lp: dict[str, float] = {
            w: math.log((c + k) / bg_total) for w, c in background.items()
        }
        self._bg_unk = math.log(k / bg_total)
        self._area_lp: dict[str, dict[str, float]] = {}
        self._area_unk: dict[str, float] = {}
        for a, cnt in area_counts.items():
            a_total = sum(cnt.values()) + k * vocab
            self._area_unk[a] = math.log(k / a_total)
            self._area_lp[a] = {w: math.log((c + k) / a_total) for w, c in cnt.items()}

    def token_score(self, area: str, word: str) -> float:
        """log P(word|area) - log P(word|background); >0 means word is more
        characteristic of `area` than of the corpus at large."""
        area_lp = self._area_lp[area].get(word, self._area_unk[area])
        bg_lp = self._bg_lp.get(word, self._bg_unk)
        return area_lp - bg_lp

    def doc_scores(self, tokens: list[str]) -> dict[str, float]:
        return {a: sum(self.token_score(a, w) for w in tokens) for a in self.areas}

    def predict(self, running: dict[str, float], margin: float) -> str | None:
        """Arg-max area from an accumulated score dict, or None if not confident."""
        if not running:
            return None
        ranked = sorted(running.items(), key=lambda kv: -kv[1])
        best, top = ranked[0]
        if top <= 0:
            return None
        second = ranked[1][1] if len(ranked) > 1 else 0.0
        return best if (top - second) >= margin else None


def candidate_list(
    engine: Engine, prev2: str | None, prev1: str | None, word: str, max_n: int
) -> list[str]:
    """Mirror src/candidates.rs::CandidateList::build — the list the IME shows
    while the user is mid-word. Item 0 is the typed prefix itself; the rest are
    context-re-ranked completions. Returns the candidate words (Latin)."""
    if not word:
        return []
    completions = [(w, c) for w, c in engine.complete(word, max_n * 3) if w != word]
    if prev1 is not None:
        boost = engine.predict_scores(prev2, prev1)
        completions.sort(key=lambda wc: (-boost.get(wc[0], 0.0), -wc[1]))
    items = [word]
    seen = {word}
    for w, _ in completions:
        if len(items) >= max_n:
            break
        if w not in seen:
            seen.add(w)
            items.append(w)
    return items
