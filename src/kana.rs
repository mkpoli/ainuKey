//! Voiced / loanword-aware romaji → katakana.
//!
//! [`ainconv::convert_latn_to_kana`] only knows **canonical Ainu**, whose
//! consonant inventory is `p t c k m n s h w r y` — it has no voiced obstruents.
//! So loanword syllables like `zu` (アイヌタイムズ *aynutaimuzu*) are mis-handled:
//! `ainconv`'s syllabifier doesn't see `z` as a consonant, attaches it as a coda
//! (`...muz`), and emits the literal `z`.
//!
//! This module scans the (already romaji-normalized) input, converts voiced /
//! loanword CV syllables directly to katakana, and hands the surrounding
//! canonical-Ainu runs to `ainconv`. Because a voiced onset always begins a new
//! syllable, splitting the run there is safe — each Ainu run `ainconv` sees is a
//! whole, valid sub-word.

/// A voiced / loanword onset+vowel → its katakana, or `None` for anything that
/// is not a voiced/loanword syllable (those go to `ainconv`). Mapped on the
/// acute-stripped vowel so long vowels (`zú`) work too.
fn voiced_cv(onset: char, vowel: char) -> Option<&'static str> {
    let v = match vowel {
        'á' => 'a',
        'í' => 'i',
        'ú' => 'u',
        'é' => 'e',
        'ó' => 'o',
        other => other,
    };
    Some(match (onset, v) {
        // が行
        ('g', 'a') => "ガ",
        ('g', 'i') => "ギ",
        ('g', 'u') => "グ",
        ('g', 'e') => "ゲ",
        ('g', 'o') => "ゴ",
        // ざ行
        ('z', 'a') => "ザ",
        ('z', 'i') => "ジ",
        ('z', 'u') => "ズ",
        ('z', 'e') => "ゼ",
        ('z', 'o') => "ゾ",
        // じゃ行 (j == palatal ざ row)
        ('j', 'a') => "ジャ",
        ('j', 'i') => "ジ",
        ('j', 'u') => "ジュ",
        ('j', 'e') => "ジェ",
        ('j', 'o') => "ジョ",
        // だ行 (traditional dakuten row)
        ('d', 'a') => "ダ",
        ('d', 'i') => "ヂ",
        ('d', 'u') => "ヅ",
        ('d', 'e') => "デ",
        ('d', 'o') => "ド",
        // ば行
        ('b', 'a') => "バ",
        ('b', 'i') => "ビ",
        ('b', 'u') => "ブ",
        ('b', 'e') => "ベ",
        ('b', 'o') => "ボ",
        // ゔ行 (loan)
        ('v', 'a') => "ヴァ",
        ('v', 'i') => "ヴィ",
        ('v', 'u') => "ヴ",
        ('v', 'e') => "ヴェ",
        ('v', 'o') => "ヴォ",
        // ふぁ行 (loan); `fu` itself is normalized to `hu` (→ フ) upstream.
        ('f', 'a') => "ファ",
        ('f', 'i') => "フィ",
        ('f', 'u') => "フ",
        ('f', 'e') => "フェ",
        ('f', 'o') => "フォ",
        _ => return None,
    })
}

/// Convert romaji to katakana, handling voiced/loanword syllables `ainconv`
/// cannot, and delegating canonical-Ainu runs to `ainconv`.
pub fn convert(latn: &str) -> String {
    let chars: Vec<char> = latn.chars().collect();
    let mut out = String::new();
    let mut run = String::new(); // accumulates a canonical-Ainu run for ainconv

    let flush = |run: &mut String, out: &mut String| {
        if !run.is_empty() {
            out.push_str(&ainconv::convert_latn_to_kana(run));
            run.clear();
        }
    };

    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // A voiced/loanword onset directly followed by a vowel is a new syllable
        // ainconv can't render — emit it here and flush the Ainu run before it.
        if i + 1 < chars.len() {
            if let Some(kana) = voiced_cv(c, chars[i + 1]) {
                flush(&mut run, &mut out);
                out.push_str(kana);
                i += 2;
                continue;
            }
        }
        run.push(c);
        i += 1;
    }
    flush(&mut run, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::convert;

    #[test]
    fn loanword_voiced_zu() {
        // The motivating case: アイヌタイムズ.
        assert_eq!(convert("aynutaimuzu"), "アイヌタイムズ");
    }

    #[test]
    fn canonical_ainu_unaffected() {
        // No voiced consonants → identical to plain ainconv.
        assert_eq!(convert("aynu"), ainconv::convert_latn_to_kana("aynu"));
        assert_eq!(
            convert("irankarapte"),
            ainconv::convert_latn_to_kana("irankarapte")
        );
        assert_eq!(convert("kamuy"), ainconv::convert_latn_to_kana("kamuy"));
    }

    #[test]
    fn voiced_rows() {
        assert_eq!(convert("ga"), "ガ");
        assert_eq!(convert("za"), "ザ");
        assert_eq!(convert("da"), "ダ");
        assert_eq!(convert("ba"), "バ");
        assert_eq!(convert("zu"), "ズ");
        assert_eq!(convert("ji"), "ジ");
    }

    #[test]
    fn mixed_run_splits_at_voiced_onset() {
        // "ta" (Ainu) + "zu" (voiced) → タズ; the run before the voiced onset is
        // converted by ainconv, the voiced syllable directly.
        assert_eq!(convert("tazu"), "タズ");
        // voiced onset mid-word, Ainu after it
        assert_eq!(convert("bake"), "バケ");
    }

    #[test]
    fn voiced_long_vowel() {
        // acute long vowel after a voiced onset
        assert_eq!(convert("zá"), "ザ");
    }

    #[test]
    fn trailing_voiced_consonant_without_vowel_is_passed_through() {
        // 'z' with no following vowel isn't a voiced syllable; ainconv handles it.
        assert_eq!(convert("az"), ainconv::convert_latn_to_kana("az"));
    }
}
