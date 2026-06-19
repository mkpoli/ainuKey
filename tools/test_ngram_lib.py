#!/usr/bin/env python3
"""Faithfulness tests: the Python benchmark engine must reproduce the Rust
IME's ranking, or the benchmark numbers mean nothing. Each case below mirrors a
specific assertion in the Rust unit tests, using the same synthetic tables.

Run:  uv run tools/test_ngram_lib.py   (plain asserts, no pytest needed)
"""
from __future__ import annotations

from ngram_lib import BlendModel, Model, candidate_list, tokenize


def synth_suggest() -> Model:
    """The table from src/suggest.rs tests (ne/wa + ruwe/ne bigrams, ruwe-ne trigram)."""
    m = Model(
        unigrams=[("ne", 100), ("wa", 50)],
        bigrams={"ruwe": [("ne", 17635)], "ne": [("na", 5522)]},
        trigrams={("ruwe", "ne"): [("na", 1200), ("wa", 900)]},
    )
    m._index()
    return m


def synth_candidates() -> Model:
    """The table from src/candidates.rs tests: bigram ku→kamui, trigram (a,ku)→kamuy."""
    m = Model(
        unigrams=[("kamuy", 1000), ("kamui", 400), ("kane", 300), ("kar", 200)],
        bigrams={"ku": [("kamui", 500)]},
        trigrams={("a", "ku"): [("kamuy", 900)]},
    )
    m._index()
    return m


def pos(items, w):
    return items.index(w)


def test_predict_scores_blends_trigram_and_bigram():
    # Mirrors suggest.rs::predict_scores_blends_trigram_and_bigram.
    s = synth_suggest()
    sc = s.predict_scores("ruwe", "ne")
    na, wa = sc.get("na", 0.0), sc.get("wa", 0.0)
    assert na > wa, f"na={na} wa={wa}"
    assert na > 1.0, f"bigram should add to na: {na}"
    assert "na" in s.predict_scores(None, "ne")
    assert s.predict_scores("x", "y") == {}


def test_predict_backoff_matches_rust():
    # Mirrors suggest.rs::predict_backs_off_trigram_to_bigram (rank order).
    s = synth_suggest()
    sc = s.predict_scores("ruwe", "ne")
    top = max(sc, key=sc.get)
    assert top == "na", top
    # prev2 unknown → bigram of "ruwe" → ne dominates
    assert max(s.predict_scores("foo", "ruwe"), key=s.predict_scores("foo", "ruwe").get) == "ne"


def test_completion_frequency_order():
    # Mirrors suggest.rs::complete_prefix_matching.
    s = synth_candidates()
    assert s.complete("kam", 8) == [("kamuy", 1000), ("kamui", 400)]
    assert s.complete("ka", 1) == [("kamuy", 1000)]
    assert s.complete("xyz", 8) == []
    assert s.complete("", 8) == []


def test_candidate_list_context_rerank():
    # Mirrors candidates.rs::{context_reranks_completions, trigram_overrides_bigram}.
    s = synth_candidates()
    # No context → frequency order: kamuy before kamui.
    plain = candidate_list(s, None, None, "kam", 8)
    assert pos(plain, "kamuy") < pos(plain, "kamui")
    assert plain[0] == "kam"  # the typed prefix is item 0
    # Bigram ku→kamui surfaces kamui before kamuy.
    bi = candidate_list(s, None, "ku", "kam", 8)
    assert pos(bi, "kamui") < pos(bi, "kamuy")
    # Trigram (a,ku)→kamuy overrides the bigram.
    tri = candidate_list(s, "a", "ku", "kam", 8)
    assert pos(tri, "kamuy") < pos(tri, "kamui")


def test_candidate_list_dedup_and_max():
    s = synth_candidates()
    c = candidate_list(s, None, None, "ka", 2)
    assert len(c) == 2
    assert len(set(c)) == len(c)  # no duplicates
    assert candidate_list(s, None, None, "", 8) == []  # empty word → empty


def test_blend_keeps_global_coverage():
    # A blend must still know every global word (coverage) and rank an in-area
    # word higher than global alone would.
    glob = Model(unigrams=[("kane", 1000), ("kamuy", 50)], bigrams={}, trigrams={})
    glob._index()
    area = Model(unigrams=[("kamuy", 200)], bigrams={}, trigrams={})
    area._index()
    blend = BlendModel(area, glob, uni_boost=6.0)
    assert "kane" in blend and "kamuy" in blend  # coverage retained
    comp = blend.complete("ka", 8)
    # global alone ranks kane(1000) >> kamuy(50); the area boost (200*6=1200)
    # lifts kamuy above kane.
    assert comp[0][0] == "kamuy", comp


def test_tokenize_matches_builder_rules():
    # Sanity: parens folded, punctuation stripped, clitics kept, digits dropped.
    assert tokenize("A(n)-ycaro, ne!") == ["an-ycaro", "ne"]
    assert tokenize("k=an wa") == ["k=an", "wa"]
    assert tokenize("123 foo7 bar") == ["bar"]  # tokens with digits dropped


def main() -> int:
    tests = [v for k, v in sorted(globals().items()) if k.startswith("test_")]
    for t in tests:
        t()
        print(f"  ok  {t.__name__}")
    print(f"\n{len(tests)} faithfulness tests passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
