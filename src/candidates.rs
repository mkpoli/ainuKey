//! Candidate-list logic for the suggestion UI: turn the current romaji word into
//! a ranked, selectable list of candidates using the [`crate::suggest`] engine.
//!
//! This is the *brain* of the candidate feature and is deliberately free of any
//! Windows/GUI code, so it unit-tests on any host. The candidate **window**
//! (rendering + caret positioning) consumes a [`CandidateList`]; the key handler
//! drives selection (`select_next`/`select_prev`/`select_index`) and commits
//! [`CandidateList::current`].
//!
//! Candidates are canonical lowercase Latin; the window converts the chosen one
//! to katakana with `ainconv` for display/commit.
//!
//! Lands ahead of its consumer (the candidate window + key integration).
#![allow(dead_code)]

use std::collections::HashSet;

use crate::suggest::Suggestions;

/// A ranked, selectable candidate list for the current composition word.
#[derive(Debug, Default, Clone)]
pub struct CandidateList {
    /// Candidate words (Latin), best first. Index 0 is the typed word itself.
    items: Vec<String>,
    /// The highlighted index.
    selected: usize,
}

impl CandidateList {
    /// Build candidates for an already-normalized romaji `word` (see
    /// [`crate::romaji::normalize`]). The word itself is candidate 0, followed by
    /// frequency-ranked completions (e.g. `kam` → `kam`, `kamuy`, `kamui`, …).
    ///
    /// When the preceding committed words (`prev2`, `prev1`) are given,
    /// completions likely to *follow* them are surfaced first — the trigram model
    /// (`prev2 prev1 → next`, backing off to the bigram `prev1 → next`) re-ranks
    /// the completions for context-aware prediction. Empty `word` yields an empty
    /// list.
    pub fn build(
        prev2: Option<&str>,
        prev1: Option<&str>,
        word: &str,
        suggest: &Suggestions,
        max: usize,
    ) -> Self {
        if word.is_empty() {
            return Self::default();
        }

        // Frequency-ranked completions of the current word (over-fetch so the
        // context re-rank has material to work with).
        let mut completions: Vec<(String, u32)> = suggest
            .complete(word, max * 3)
            .into_iter()
            .filter(|(w, _)| *w != word)
            .map(|(w, c)| (w.to_string(), c))
            .collect();

        // Context boost: prefer completions the trigram/bigram model predicts
        // after (prev2, prev1).
        if let Some(prev1) = prev1 {
            let boost: std::collections::HashMap<&str, u32> = suggest
                .predict(prev2, prev1)
                .iter()
                .map(|(w, c)| (w.as_str(), *c))
                .collect();
            // Stable order: predicted-next completions first (by predict weight),
            // then the remaining completions by their own frequency.
            completions.sort_by(|a, b| {
                let ba = boost.get(a.0.as_str()).copied().unwrap_or(0);
                let bb = boost.get(b.0.as_str()).copied().unwrap_or(0);
                bb.cmp(&ba).then_with(|| b.1.cmp(&a.1))
            });
        }

        let mut items = Vec::with_capacity(max);
        let mut seen = HashSet::new();
        items.push(word.to_string());
        seen.insert(word.to_string());
        for (w, _) in completions {
            if items.len() >= max {
                break;
            }
            if seen.insert(w.clone()) {
                items.push(w);
            }
        }
        Self { items, selected: 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn items(&self) -> &[String] {
        &self.items
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    /// The currently highlighted candidate, if any.
    pub fn current(&self) -> Option<&str> {
        self.items.get(self.selected).map(String::as_str)
    }

    /// Move the highlight down one, wrapping. No-op if empty.
    pub fn select_next(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + 1) % self.items.len();
        }
    }

    /// Move the highlight up one, wrapping. No-op if empty.
    pub fn select_prev(&mut self) {
        if !self.items.is_empty() {
            self.selected = (self.selected + self.items.len() - 1) % self.items.len();
        }
    }

    /// Select by 0-based index (e.g. number-key choice). Returns false if out of
    /// range (selection unchanged).
    pub fn select_index(&mut self, index: usize) -> bool {
        if index < self.items.len() {
            self.selected = index;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A small synthetic table: unigrams kamuy/kamui/kane/kar, plus a bigram
    /// `ku` → kamui (so context can override raw frequency).
    fn suggest() -> Suggestions {
        let mut b = Vec::new();
        b.extend(b"AKNG");
        b.extend(2u32.to_le_bytes()); // version
        let uni = [
            ("kamuy", 1000u32),
            ("kamui", 400),
            ("kane", 300),
            ("kar", 200),
        ];
        b.extend((uni.len() as u32).to_le_bytes());
        for (w, c) in uni {
            b.push(w.len() as u8);
            b.extend(w.as_bytes());
            b.extend(c.to_le_bytes());
        }
        // bigrams: one context "ku" -> [("kamui", 500)]
        b.extend(1u32.to_le_bytes());
        b.push(2);
        b.extend(b"ku");
        b.push(1); // one next-word
        b.push(5);
        b.extend(b"kamui");
        b.extend(500u32.to_le_bytes());
        // trigrams: one context "a ku" -> [("kamuy", 900)] (overrides the bigram).
        b.extend(1u32.to_le_bytes());
        b.push(4);
        b.extend(b"a ku");
        b.push(1);
        b.push(5);
        b.extend(b"kamuy");
        b.extend(900u32.to_le_bytes());
        Suggestions::load(&b).expect("synth")
    }

    fn pos(c: &CandidateList, w: &str) -> usize {
        c.items().iter().position(|x| x == w).expect("present")
    }

    #[test]
    fn typed_word_is_first_then_completions() {
        let s = suggest();
        let c = CandidateList::build(None, None, "kam", &s, 8);
        assert_eq!(c.items()[0], "kam"); // the typed word
        assert!(c.items().contains(&"kamuy".to_string()));
        assert!(c.items().contains(&"kamui".to_string()));
        // 'kane'/'kar' do not start with 'kam', so are excluded.
        assert!(!c.items().contains(&"kane".to_string()));
    }

    #[test]
    fn context_reranks_completions() {
        let s = suggest();
        // No context → frequency order: kamuy (1000) before kamui (400).
        let plain = CandidateList::build(None, None, "kam", &s, 8);
        assert!(pos(&plain, "kamuy") < pos(&plain, "kamui"));
        // prev1 = "ku" (bigram) predicts kamui → kamui surfaces before kamuy.
        let ctx = CandidateList::build(None, Some("ku"), "kam", &s, 8);
        assert!(pos(&ctx, "kamui") < pos(&ctx, "kamuy"));
    }

    #[test]
    fn trigram_context_overrides_bigram() {
        let s = suggest();
        // Bigram "ku" → kamui, but trigram "a ku" → kamuy. With both prevs the
        // trigram wins: kamuy surfaces before kamui.
        let bi = CandidateList::build(None, Some("ku"), "kam", &s, 8);
        assert!(pos(&bi, "kamui") < pos(&bi, "kamuy"));
        let tri = CandidateList::build(Some("a"), Some("ku"), "kam", &s, 8);
        assert!(pos(&tri, "kamuy") < pos(&tri, "kamui"));
    }

    #[test]
    fn dedups_typed_word_against_completion() {
        let s = suggest();
        // 'kamuy' is both the typed word and a known unigram; it appears once.
        let c = CandidateList::build(None, None, "kamuy", &s, 8);
        assert_eq!(c.items().iter().filter(|w| *w == "kamuy").count(), 1);
        assert_eq!(c.items()[0], "kamuy");
    }

    #[test]
    fn respects_max() {
        let s = suggest();
        let c = CandidateList::build(None, None, "ka", &s, 2);
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn empty_word_is_empty_list() {
        let s = suggest();
        assert!(CandidateList::build(None, None, "", &s, 8).is_empty());
    }

    #[test]
    fn selection_wraps_and_indexes() {
        let s = suggest();
        let mut c = CandidateList::build(None, None, "ka", &s, 8);
        let n = c.len();
        assert_eq!(c.selected(), 0);
        c.select_prev();
        assert_eq!(c.selected(), n - 1); // wrap up from 0
        c.select_next();
        assert_eq!(c.selected(), 0); // wrap back down
        assert!(c.select_index(n - 1));
        assert_eq!(c.current(), Some(c.items()[n - 1].as_str()));
        assert!(!c.select_index(n)); // out of range
    }
}
