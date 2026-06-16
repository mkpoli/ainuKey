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

use crate::config::{Orthography, TuStyle};

/// Vowels, including the acute long forms `ainconv` uses.
fn is_vowel(c: char) -> bool {
    matches!(c, 'a' | 'i' | 'u' | 'e' | 'o' | 'á' | 'í' | 'ú' | 'é' | 'ó')
}

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

/// Delegate a canonical-Ainu run to `ainconv`, defensively.
///
/// `ainconv` (0.2.0) panics on a word whose syllable ends in the multibyte
/// glottal stop `’` (U+2019): its syllabifier does `split_at(len - 1)`, slicing
/// mid-character. Such words occur in the corpus (so they appear as candidate
/// completions). We normalize `’` to an ASCII apostrophe first — `ainconv`
/// strips that safely and the glottal isn't rendered in katakana either way —
/// and additionally catch any residual panic so a dependency bug can never crash
/// the host application (a panic across the TSF/COM boundary would).
fn convert_run(run: &str) -> String {
    let safe = run.replace('\u{2019}', "'");
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        ainconv::convert_latn_to_kana(&safe)
    }))
    .unwrap_or(safe)
}

/// Convert romaji to katakana with the default notation options. A stable
/// convenience entry point; the IME passes explicit options via `convert_with`,
/// so this is currently used only by tests.
#[allow(dead_code)]
pub fn convert(latn: &str) -> String {
    convert_with(latn, &Orthography::default())
}

/// Convert romaji to katakana, applying notation options on top of the default
/// conversion:
/// * `show_equals_boundary` — keep the `=` morpheme boundary (ainconv strips it),
/// * `small_glides` — render coda `y`/`w` as small ィ/ゥ (ainconv collapses to イ/ウ),
/// * `tu_style` — render the `tu` mora as ツ゚ instead of ト゚.
pub fn convert_with(latn: &str, ortho: &Orthography) -> String {
    let chars: Vec<char> = latn.chars().collect();
    let mut out = String::new();
    let mut run = String::new(); // accumulates a canonical-Ainu run for ainconv

    let flush = |run: &mut String, out: &mut String| {
        if !run.is_empty() {
            // ainconv's syllabifier underflows (panics in debug builds) on a
            // vowel-less run; such a run is just an in-progress consonant cluster
            // (e.g. the user has typed only `k`), so emit it verbatim instead of
            // delegating — it becomes katakana once a vowel completes the syllable.
            if run.chars().any(is_vowel) {
                out.push_str(&convert_run(run));
            } else {
                out.push_str(run);
            }
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
        // `=` morpheme boundary: ainconv strips it, so decide here.
        if c == '=' {
            flush(&mut run, &mut out);
            if ortho.show_equals_boundary {
                out.push('=');
            }
            i += 1;
            continue;
        }
        // Coda glide: `y`/`w` after a vowel and not before a vowel. ainconv
        // collapses these to イ/ウ; with `small_glides` we keep the small kana.
        if ortho.small_glides
            && (c == 'y' || c == 'w')
            && i > 0
            && is_vowel(chars[i - 1])
            && (i + 1 >= chars.len() || !is_vowel(chars[i + 1]))
        {
            flush(&mut run, &mut out);
            out.push(if c == 'y' { 'ィ' } else { 'ゥ' });
            i += 1;
            continue;
        }
        run.push(c);
        i += 1;
    }
    flush(&mut run, &mut out);

    // `tu` is the only source of ト゚, so this swap is unambiguous.
    if ortho.tu_style == TuStyle::Tsu {
        out = out.replace("ト゚", "ツ゚");
    }
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

    // ---- Full voiced/loanword row coverage. ----

    #[test]
    fn g_row() {
        assert_eq!(convert("ga"), "ガ");
        assert_eq!(convert("gi"), "ギ");
        assert_eq!(convert("gu"), "グ");
        assert_eq!(convert("ge"), "ゲ");
        assert_eq!(convert("go"), "ゴ");
    }

    #[test]
    fn z_row() {
        assert_eq!(convert("za"), "ザ");
        assert_eq!(convert("zi"), "ジ");
        assert_eq!(convert("zu"), "ズ");
        assert_eq!(convert("ze"), "ゼ");
        assert_eq!(convert("zo"), "ゾ");
    }

    #[test]
    fn j_row() {
        assert_eq!(convert("ja"), "ジャ");
        assert_eq!(convert("ji"), "ジ");
        assert_eq!(convert("ju"), "ジュ");
        assert_eq!(convert("je"), "ジェ");
        assert_eq!(convert("jo"), "ジョ");
    }

    #[test]
    fn d_row() {
        assert_eq!(convert("da"), "ダ");
        assert_eq!(convert("di"), "ヂ");
        assert_eq!(convert("du"), "ヅ");
        assert_eq!(convert("de"), "デ");
        assert_eq!(convert("do"), "ド");
    }

    #[test]
    fn b_row() {
        assert_eq!(convert("ba"), "バ");
        assert_eq!(convert("bi"), "ビ");
        assert_eq!(convert("bu"), "ブ");
        assert_eq!(convert("be"), "ベ");
        assert_eq!(convert("bo"), "ボ");
    }

    #[test]
    fn v_row() {
        assert_eq!(convert("va"), "ヴァ");
        assert_eq!(convert("vi"), "ヴィ");
        assert_eq!(convert("vu"), "ヴ");
        assert_eq!(convert("ve"), "ヴェ");
        assert_eq!(convert("vo"), "ヴォ");
    }

    #[test]
    fn f_row() {
        // `fu` itself is normalized to `hu` upstream, but bare `fu` here still
        // maps to フ; fa/fi/fe/fo are the loan forms.
        assert_eq!(convert("fa"), "ファ");
        assert_eq!(convert("fi"), "フィ");
        assert_eq!(convert("fu"), "フ");
        assert_eq!(convert("fe"), "フェ");
        assert_eq!(convert("fo"), "フォ");
    }

    #[test]
    fn multiple_voiced_syllables_in_one_word() {
        assert_eq!(convert("gaza"), "ガザ");
        assert_eq!(convert("bobo"), "ボボ");
        assert_eq!(convert("gazabado"), "ガザバド");
    }

    #[test]
    fn voiced_at_start_middle_end() {
        // start
        assert_eq!(convert("zaki"), "ザキ");
        // middle (Ainu run on both sides)
        assert_eq!(convert("kaba"), "カバ");
        // end (the motivating shape)
        assert_eq!(convert("kamuzu"), "カムズ");
    }

    #[test]
    fn real_loanwords() {
        assert_eq!(convert("rajio"), "ラジオ"); // ラジオ (radio)
        assert_eq!(convert("terebi"), "テレビ"); // テレビ (TV)
        assert_eq!(convert("bideo"), "ビデオ"); // ビデオ (video)
        assert_eq!(convert("garasu"), "ガラス"); // ガラス (glass)
    }

    #[test]
    fn voiced_long_vowels_all() {
        assert_eq!(convert("gá"), "ガ");
        assert_eq!(convert("zú"), "ズ");
        assert_eq!(convert("bó"), "ボ");
    }

    #[test]
    fn empty_and_single_char() {
        assert_eq!(convert(""), "");
        assert_eq!(convert("a"), "ア");
        // A voiced onset that does start a syllable still maps directly.
        assert_eq!(convert("zo"), "ゾ");
    }

    // ---- Cross-module pipeline: romaji::normalize THEN kana::convert. ----
    // This is the exact transform the IME applies to the buffer.

    #[test]
    fn pipeline_canonical_words() {
        let pipe = |s: &str| convert(&crate::romaji::normalize(s));
        assert_eq!(pipe("irankarapte"), "イランカラㇷ゚テ");
        assert_eq!(pipe("aynu"), "アイヌ");
        assert_eq!(pipe("kamuy"), "カムイ");
    }

    #[test]
    fn pipeline_loanword_voiced() {
        let pipe = |s: &str| convert(&crate::romaji::normalize(s));
        assert_eq!(pipe("aynutaimuzu"), "アイヌタイムズ"); // アイヌタイムズ
        assert_eq!(pipe("AYNUTAIMUZU"), "アイヌタイムズ"); // case-folded too
    }

    #[test]
    fn pipeline_voiced_plus_digraph() {
        // normalize folds the legacy digraph, then convert handles the voiced CV.
        let pipe = |s: &str| convert(&crate::romaji::normalize(s));
        assert_eq!(pipe("chibi"), "チビ"); // chi→ci (R3), bi voiced
        assert_eq!(pipe("Fuji"), "フジ"); // fu→hu (R8), ji voiced
    }

    #[test]
    fn vowelless_run_is_emitted_verbatim_without_panic() {
        // A run with no vowel (in-progress consonant cluster) must not reach
        // ainconv, whose syllabifier underflows on it. It is shown as-is until a
        // vowel completes the syllable.
        assert_eq!(convert("k"), "k");
        assert_eq!(convert("n"), "n");
        assert_eq!(convert("ng"), "ng");
        // and once the vowel arrives it converts normally
        assert_eq!(convert("ka"), "カ");
        assert_eq!(convert("na"), "ナ");
    }

    #[test]
    fn glottal_stop_does_not_crash() {
        // Regression: ainconv 0.2.0 panics on a syllable ending in the multibyte
        // glottal stop ’ (U+2019) — split_at(len-1) slices mid-char. Such words
        // appear as corpus candidate completions, so rendering them used to crash
        // the host app (typing "na" surfaced one). convert_run must absorb it.
        assert_eq!(convert("ne\u{2019}"), "ネ");
        assert_eq!(convert("po\u{2019}"), "ポ");
        assert_eq!(convert("a\u{2019}e"), "アエ");
        // glottal mid-word, and as a bare/leading char — must not panic
        let _ = convert("\u{2019}");
        let _ = convert("nispa\u{2019}");
    }

    // ---- Orthography / notation options (convert_with). ----
    use super::convert_with;
    use crate::config::{Orthography, TuStyle};

    fn ortho(f: impl FnOnce(&mut Orthography)) -> Orthography {
        let mut o = Orthography::default();
        f(&mut o);
        o
    }

    #[test]
    fn default_orthography_matches_plain_convert() {
        for w in ["aynutaimuzu", "kamuy", "tumpu", "a=kor", "irankarapte"] {
            assert_eq!(convert_with(w, &Orthography::default()), convert(w), "{w}");
        }
    }

    #[test]
    fn tu_style_tsu() {
        // ainconv renders `tu` as ト゚; the Tsu option swaps it to ツ゚.
        assert_eq!(convert("tu"), "ト゚");
        assert_eq!(
            convert_with("tu", &ortho(|o| o.tu_style = TuStyle::Tsu)),
            "ツ゚"
        );
        assert_eq!(
            convert_with("tumpu", &ortho(|o| o.tu_style = TuStyle::Tsu)),
            "ツ゚ㇺプ"
        );
    }

    #[test]
    fn small_glides() {
        // Coda y/w collapse to イ/ウ by default; the option keeps the small kana.
        assert_eq!(convert("kamuy"), "カムイ");
        assert_eq!(
            convert_with("kamuy", &ortho(|o| o.small_glides = true)),
            "カムィ"
        );
        assert_eq!(
            convert_with("ay", &ortho(|o| o.small_glides = true)),
            "アィ"
        );
        // onset y/w (followed by a vowel) is unaffected
        assert_eq!(convert_with("ya", &ortho(|o| o.small_glides = true)), "ヤ");
        assert_eq!(
            convert_with("yaywa", &ortho(|o| o.small_glides = true)),
            "ヤィワ"
        );
    }

    #[test]
    fn equals_boundary() {
        // ainconv strips `=`; with the option it's kept between the katakana.
        assert_eq!(convert("a=kor"), "アコㇿ"); // default: stripped
        assert_eq!(
            convert_with("a=kor", &ortho(|o| o.show_equals_boundary = true)),
            "ア=コㇿ"
        );
        assert_eq!(
            convert_with("ku=kor", &ortho(|o| o.show_equals_boundary = true)),
            "ク=コㇿ"
        );
    }
}
