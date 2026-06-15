//! Forgiving-input → canonical-Ainu-romaji normalization.
//!
//! This module pre-processes the user's raw IME buffer *before* it is handed to
//! [`ainconv::convert_latn_to_kana`]. The call site is:
//!
//! ```ignore
//! let kana = ainconv::convert_latn_to_kana(&crate::romaji::normalize(&buffer));
//! ```
//!
//! ## Why this exists
//!
//! `ainconv` is the ground truth for kana output, and it already normalizes a
//! number of things internally (verified against the `ainconv` 0.2.0 source,
//! `convert_word()` in `src/conversion/katakana.rs`):
//!
//! * `=` (morpheme boundary) is stripped before lookup.
//! * Acute accents are stripped before CV lookup (`remove_acute_accent`), so
//!   length is irrelevant to the kana — `káni` and `kani` both → カニ.
//! * Input is `to_ascii_lowercase`d — **ASCII** case only.
//! * The glottal stop is accepted as either ASCII `'` (U+0027) or `’` (U+2019).
//! * Syllabification, coda placement, `nn`→ン and gemination are all owned by
//!   the syllabifier. We must never touch `n`, codas, or doubled letters.
//!
//! `ainconv` does **NOT** understand:
//!
//! * Macron (`ā`) or circumflex (`â`) long vowels — these are opaque to the
//!   acute-stripper and make the converter error (`kāni` → error). This is the
//!   one long-vowel notation we genuinely must rewrite.
//! * Non-ASCII *uppercase* acute vowels (`Á`) — `to_ascii_lowercase` skips them,
//!   so we lowercase with full Unicode folding to be safe.
//! * Legacy / Hepburn-style digraphs (`chi`, `shi`, `ch`, `ti`, `fu`) that real
//!   users type. `ainconv` errors on these (e.g. raw `ti` → "cannot find
//!   katakana for CV pair: 'ti'").
//!
//! So `normalize()` does exactly five kinds of work, in this order:
//!
//! 1. Lowercase with full Unicode folding (`Á` → `á`, `AYNU` → `aynu`).
//! 2. Map macron long vowels (`ā ī ū ē ō`) → acute (`á í ú é ó`).
//! 3. Map circumflex long vowels (`â î û ê ô`) → acute (`á í ú é ó`).
//! 4. Apply the vetted digraph rules in one left-to-right, longest-match-first,
//!    non-overlapping pass.
//! 5. (No-op for storage.) We deliberately do **not** rewrite ASCII `'` → `’`:
//!    `ainconv` accepts both, so it is unnecessary for correctness.
//!
//! ## The digraph rules — what survived the adversarial audit
//!
//! Canonical Ainu inventory (from `ainconv`'s `latin.rs`):
//! * consonants: `p t c k m n s h w y ’` (affricate is `c`; no `b d g z f j`)
//! * vowels: `a i u e o` (+ acute-long `á í ú é ó`)
//!
//! | rule | from | to  | status | reason |
//! |------|------|-----|--------|--------|
//! | R3   | `chi`| `ci`| KEEP   | `c` is never a coda, so `ch` can only be a legacy チ digraph; no canonical collision. |
//! | R5   | `ch` | `c` | KEEP   | same — generalizes `cha/chu/che/cho` → `ca/cu/ce/co`. |
//! | R7   | `shi`| `si`| KEEP, onset-gated | シ is canonically `si`. The only risk is a coda-`s` + `h`-onset seam (cf. `pethat`/`sathat` for the `t` analogue); gating to true onset avoids it. |
//! | R4   | `ti` | `ci`| KEEP, onset-gated | Canonical Ainu has **no** morpheme-internal `ti`; the affricate is always `ci`. Risk is a coda-`t` + `i`-onset seam; gating to true onset avoids the cluster case. (The explicitly requested feature.) |
//! | R8   | `fu` | `hu`| KEEP   | フ is canonically `hu`; `f` is not in the inventory and never a coda, so `fu` is only ever the legacy フ spelling. |
//! | R6   | `tsu`| `tu` | **DROP** | RISKY / corrupting. `ts` is a legal **coda-`t` + onset-`s`** sequence in canonical orthography: `satsuwe` サッスウェ (`sat`+`suwe`), `petsuwe`, `matsuwop`, `sutsurke`, `enotsuye` are real dictionary headwords. `satsuwe`→`satuwe` would wrongly give サト゚ウェ. The ツ sound is canonically typed `tu` directly, so no rule is needed. |
//!
//! Also explicitly **excluded** (would corrupt valid Ainu): blanket `sh`→`s`,
//! blanket `f`→`h`, `wo`→`o` (real syllable ヲ/ウォ), `ji`/`j`→`ci`, `aa`→`á`
//! (real two-mora hiatus), `di`/`du`, any `n` rewriting, voiced `b d g z`.
//!
//! ## Onset gating (R4 `ti`, R7 `shi`)
//!
//! The first letter of these digraphs (`t` / `s`) *can* be a coda in canonical
//! Ainu, so a blind global replace can fire across a syllable seam. We restrict
//! R4 and R7 to **true syllable onset**: the leading consonant must be at a word
//! boundary or immediately preceded by a vowel or `’` — i.e. NOT itself the tail
//! of a consonant cluster (which would make it a coda). This protects the
//! cluster seams (`...Ct` + `i...`, `...Cs` + `hi...`) while still folding the
//! overwhelmingly common mistyped-チ/シ case. R3/R5/R8 need no gate because
//! `c` and `f` are never codas in Ainu.
//!
//! Because we run on the raw buffer (with `=` still present), a `=` between the
//! two letters (`t=i`) is a literal break and is never matched — so we never
//! fold across an explicit morpheme boundary.
//!
//! Pure `std` only — this module must not depend on `ainconv` or any crate.

/// Canonical Ainu vowels, including the acute long vowels. Used by the
/// onset-gating check (R4/R7): a consonant is in onset position if it sits at a
/// word boundary or right after a vowel or `’` (so it is not a coda / not the
/// tail of a consonant cluster).
const VOWELS: &[char] = &['a', 'i', 'u', 'e', 'o', 'á', 'í', 'ú', 'é', 'ó'];

/// Map a macron or circumflex long vowel to its acute equivalent.
///
/// `ainconv` strips acute before CV lookup, so acute is the long-vowel notation
/// it understands; macron/circumflex make it error. Returns `Some(acute)` for a
/// macron/circumflex vowel, `None` for anything else.
fn long_vowel_to_acute(c: char) -> Option<char> {
    match c {
        // R1: macron → acute
        'ā' => Some('á'),
        'ī' => Some('í'),
        'ū' => Some('ú'),
        'ē' => Some('é'),
        'ō' => Some('ó'),
        // R2: circumflex → acute
        'â' => Some('á'),
        'î' => Some('í'),
        'û' => Some('ú'),
        'ê' => Some('é'),
        'ô' => Some('ó'),
        _ => None,
    }
}

/// Is `c` a canonical Ainu vowel (short or acute-long)?
fn is_vowel(c: char) -> bool {
    VOWELS.contains(&c)
}

/// Normalize a raw Ainu IME buffer into canonical romaji that
/// [`ainconv::convert_latn_to_kana`] accepts.
///
/// Phases, in order:
/// 1. Lowercase with full Unicode folding (handles `AYNU` and `Á`/`É`…).
/// 2. Macron/circumflex long vowels → acute (single-codepoint substitution).
/// 3. Vetted digraph pass, longest-match-first, single non-overlapping left-to-
///    right scan: `chi`→`ci`, `shi`→`si` (gated), `ch`→`c`, `ti`→`ci` (gated),
///    `fu`→`hu`. `tsu` is intentionally NOT handled.
///
/// Everything else — `=` stripping, acute stripping, glottal-stop handling,
/// syllabification, codas, gemination, `nn` — is left to `ainconv`.
pub fn normalize(romaji: &str) -> String {
    // --- Phase 1: lowercase (full Unicode, so non-ASCII acute caps fold too). ---
    // `char::to_lowercase` can expand one char to several; `String::to_lowercase`
    // handles that. This subsumes ainconv's ASCII-only lowercasing and also folds
    // `Á`→`á`, which ainconv would otherwise miss.
    let lowered = romaji.to_lowercase();

    // --- Phase 2: macron/circumflex → acute, char by char. ---
    // Single-codepoint substitutions; order among themselves is irrelevant.
    let folded: String = lowered
        .chars()
        .map(|c| long_vowel_to_acute(c).unwrap_or(c))
        .collect();

    // --- Phase 3: digraph pass. ---
    // One left-to-right scan over the char vector. At each position we try the
    // longest applicable rule first and, on a match, advance *past* the consumed
    // input so matches never overlap. This is what makes ordering deterministic:
    // 3-char `chi`/`shi` are tried before 2-char `ch`/`ti`, so `chi` becomes `ci`
    // (not `c`+`hi`) and `shi` becomes `si` (not `s`+`hi`).
    let chars: Vec<char> = folded.chars().collect();
    let mut out = String::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let c0 = chars[i];
        let c1 = chars.get(i + 1).copied();
        let c2 = chars.get(i + 2).copied();

        // True-onset test for the leading consonant at position `i`: it must be
        // at a word boundary or directly after a vowel, a glottal stop, or a
        // morpheme/word separator — i.e. it is the onset of a syllable, not the
        // tail of a consonant cluster (a coda). A coda consonant is, by
        // definition, preceded by another consonant; an onset never is.
        // `out`'s last pushed char is the already-normalized predecessor.
        //
        // `=` (morpheme boundary) and whitespace count as onset boundaries: the
        // consonant after them begins a fresh morpheme/word, so it is an onset.
        // (ainconv strips `=` before its own CV lookup, which would leave the
        // same onset `t`/`s` preceded by the prior morpheme's final vowel.)
        let prev_is_onset_boundary = match out.chars().next_back() {
            None => true, // word start
            Some(p) => is_vowel(p) || p == '’' || p == '\'' || p == '=' || p.is_whitespace(),
        };

        // R3 (3-char): chi -> ci. `c` is never a coda, so no gate needed.
        if c0 == 'c' && c1 == Some('h') && c2 == Some('i') {
            out.push('c');
            out.push('i');
            i += 3;
            continue;
        }

        // R7 (3-char): shi -> si. Onset-gated: only when `s` is a true onset,
        // never when it is a coda-`s` meeting an `h`-initial syllable.
        if c0 == 's' && c1 == Some('h') && c2 == Some('i') && prev_is_onset_boundary {
            out.push('s');
            out.push('i');
            i += 3;
            continue;
        }

        // R5 (2-char): ch -> c (covers cha/chu/che/cho/ch...). Tried after the
        // 3-char `chi` so `chi` is never split; `c` is never a coda → no gate.
        if c0 == 'c' && c1 == Some('h') {
            out.push('c');
            i += 2;
            continue;
        }

        // R4 (2-char): ti -> ci. Onset-gated: only when `t` is a true onset.
        // Canonical Ainu has no morpheme-internal `ti`; this is the requested
        // mistyped-チ fold. The gate protects coda-`t` + `i`-onset cluster seams.
        if c0 == 't' && c1 == Some('i') && prev_is_onset_boundary {
            out.push('c');
            out.push('i');
            i += 2;
            continue;
        }

        // R8 (2-char): fu -> hu. `f` is not in the inventory and never a coda,
        // so `fu` is only ever the legacy フ spelling → no gate needed.
        if c0 == 'f' && c1 == Some('u') {
            out.push('h');
            out.push('u');
            i += 2;
            continue;
        }

        // Default: copy through unchanged. Note `tsu` falls here (R6 dropped),
        // so `satsuwe` etc. survive verbatim for ainconv to syllabify correctly.
        out.push(c0);
        i += 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::normalize;

    // ---- Core requested feature: ti -> ci, plus the chi/shi/ch family. ----

    #[test]
    fn ti_to_ci_onset() {
        // The headline feature. Ainu has no canonical onset `ti`; fold to `ci`.
        assert_eq!(normalize("tikuni"), "cikuni"); // → チクニ
        assert_eq!(normalize("ti"), "ci");
        assert_eq!(normalize("tita"), "cita");
    }

    #[test]
    fn chi_to_ci() {
        assert_eq!(normalize("chikuni"), "cikuni"); // → チクニ
        assert_eq!(normalize("chise"), "cise"); // → チセ
        assert_eq!(normalize("chi"), "ci");
    }

    #[test]
    fn ch_family_to_c() {
        // cha/chu/che/cho → ca/cu/ce/co (ainconv: ca=チャ, ce=チェ, …).
        assert_eq!(normalize("cha"), "ca");
        assert_eq!(normalize("chu"), "cu");
        assert_eq!(normalize("che"), "ce");
        assert_eq!(normalize("cho"), "co");
        // chi must NOT be split by the 2-char ch rule — 3-char wins.
        assert_eq!(normalize("chi"), "ci");
        assert_eq!(normalize("pacha"), "paca");
    }

    #[test]
    fn shi_to_si_onset() {
        assert_eq!(normalize("shisam"), "sisam"); // → シサㇺ
        assert_eq!(normalize("shi"), "si");
        assert_eq!(normalize("ashi"), "asi"); // onset after a vowel
    }

    #[test]
    fn fu_to_hu() {
        assert_eq!(normalize("fuchi"), "huci"); // フチ (combines R8 + R3)
        assert_eq!(normalize("fu"), "hu");
    }

    // ---- The dropped rule (R6): tsu MUST be left alone. ----

    #[test]
    fn tsu_is_not_folded() {
        // Real dictionary headwords with coda-`t` + onset-`s`. Folding `tsu`→`tu`
        // would corrupt these (ainconv: `satsuwe`→サッスウェ but `satuwe`→サト゚ウェ).
        assert_eq!(normalize("satsuwe"), "satsuwe");
        assert_eq!(normalize("petsuwe"), "petsuwe");
        assert_eq!(normalize("matsuwop"), "matsuwop");
        assert_eq!(normalize("sutsurke"), "sutsurke");
        assert_eq!(normalize("sirsutsurke"), "sirsutsurke");
        assert_eq!(normalize("enotsuye"), "enotsuye");
        // And the bare cluster.
        assert_eq!(normalize("tsuki"), "tsuki");
    }

    #[test]
    fn tu_is_canonical_and_untouched() {
        // The ツ sound is typed `tu` directly; `sutu` (ストゥ) etc. stay verbatim.
        assert_eq!(normalize("tu"), "tu");
        assert_eq!(normalize("tuki"), "tuki"); // → ト゚キ
        assert_eq!(normalize("tunci"), "tunci"); // → ト゚ンチ
        assert_eq!(normalize("sutu"), "sutu"); // → ストゥ
        assert_eq!(normalize("numasut"), "numasut");
    }

    // ---- Onset gating: coda seams must NOT be folded. ----

    #[test]
    fn ti_coda_cluster_seam_not_folded() {
        // A coda-`t` that is the tail of a consonant cluster meeting `i` must
        // keep its `t`. The gate (prev char must be vowel/’/start) blocks it.
        // e.g. `...rt` + `i...`: the `t` after `r` is a coda, not an onset.
        assert_eq!(normalize("karti"), "karti");
        assert_eq!(normalize("masti"), "masti");
    }

    #[test]
    fn shi_coda_cluster_seam_not_folded() {
        // coda-`s` (cluster tail) + `hi`-onset: gate blocks the fold.
        // cf. real coda+`h` seams like `pethat` (ペッハッ), `sathat` (サッハッ).
        assert_eq!(normalize("karshi"), "karshi");
        assert_eq!(normalize("perashi"), "perasi"); // `s` after vowel `a` IS onset → folds
    }

    #[test]
    fn ti_across_morpheme_boundary() {
        // `=` is an onset boundary: a `t` right after `=` begins a morpheme, so
        // it is a genuine onset and `ti` correctly folds to `ci`. (ainconv
        // strips `=`, leaving the onset `t` after the prior vowel — same fold.)
        assert_eq!(normalize("a=ti"), "a=ci");
        // But `t=i` is not even a `ti` substring (the `=` splits them), so the
        // coda-`t` + clitic-`i` case is never touched: `at=itak` stays as-is.
        assert_eq!(normalize("at=itak"), "at=itak");
    }

    // ---- Long vowels: macron / circumflex → acute. ----

    #[test]
    fn macron_to_acute() {
        assert_eq!(normalize("kāni"), "káni"); // → カニ (raw `kāni` errors in ainconv)
        assert_eq!(normalize("tāne"), "táne");
        assert_eq!(normalize("sīno"), "síno");
        assert_eq!(normalize("tūki"), "túki");
        assert_eq!(normalize("pēka"), "péka");
        assert_eq!(normalize("pōro"), "póro");
    }

    #[test]
    fn circumflex_to_acute() {
        assert_eq!(normalize("kâni"), "káni"); // → カニ
        assert_eq!(normalize("sîno"), "síno");
        assert_eq!(normalize("tûki"), "túki");
        assert_eq!(normalize("pêka"), "péka");
        assert_eq!(normalize("pôro"), "póro");
    }

    // ---- Case folding (ASCII + non-ASCII acute caps). ----

    #[test]
    fn lowercases_ascii() {
        assert_eq!(normalize("AYNU"), "aynu"); // → アイヌ
        assert_eq!(normalize("WENKUR"), "wenkur"); // → ウェンクㇽ
        assert_eq!(normalize("Chise"), "cise"); // case + digraph
    }

    #[test]
    fn lowercases_non_ascii_acute_caps() {
        // ainconv's to_ascii_lowercase would NOT fold these; we must.
        assert_eq!(normalize("Áynu"), "áynu"); // → アイヌ
        assert_eq!(normalize("KÁNI"), "káni");
        assert_eq!(normalize("ÉRAMAN"), "éraman");
    }

    #[test]
    fn lowercases_macron_caps() {
        // Uppercase macron should lowercase (Phase 1) then map to acute (Phase 2).
        assert_eq!(normalize("KĀNI"), "káni");
    }

    // ---- Things we must NOT touch (excluded rules). ----

    #[test]
    fn wo_is_preserved() {
        // `wo` is a real canonical syllable (ヲ → ウォ). Never collapse to `o`.
        assert_eq!(normalize("wose"), "wose"); // → ウォセ
        assert_eq!(normalize("wo"), "wo");
    }

    #[test]
    fn double_vowel_is_preserved() {
        // Two morae / hiatus is real; do not collapse to a long vowel.
        assert_eq!(normalize("aa"), "aa"); // → アア
        assert_eq!(normalize("koonkami"), "koonkami");
    }

    #[test]
    fn n_handling_is_left_to_ainconv() {
        assert_eq!(normalize("pon"), "pon");
        assert_eq!(normalize("ponno"), "ponno"); // → ポンノ
        assert_eq!(normalize("tanne"), "tanne");
        assert_eq!(normalize("nispa"), "nispa"); // → ニㇱパ
    }

    #[test]
    fn glottal_stop_both_forms_passthrough() {
        // ainconv accepts both `'` and `’`; we don't rewrite either.
        assert_eq!(normalize("poro'an"), "poro'an"); // → ポロアン
        assert_eq!(normalize("poro’an"), "poro’an"); // → ポロアン
    }

    #[test]
    fn voiced_and_foreign_letters_passthrough() {
        // No `b d g z j` mapping — leave them so ainconv surfaces the problem
        // rather than us silently guessing. (Only `fu`, not blanket `f`.)
        assert_eq!(normalize("di"), "di");
        assert_eq!(normalize("du"), "du");
        assert_eq!(normalize("ji"), "ji");
        assert_eq!(normalize("baka"), "baka");
    }

    // ---- Canonical words that should pass through unchanged. ----

    #[test]
    fn canonical_words_unchanged() {
        assert_eq!(normalize("huci"), "huci"); // → フチ
        assert_eq!(normalize("cise"), "cise"); // → チセ
        assert_eq!(normalize("cikuni"), "cikuni"); // → チクニ
        assert_eq!(normalize("kamuy"), "kamuy"); // → カムイ
        assert_eq!(normalize("cep"), "cep"); // → チェㇷ゚
        assert_eq!(normalize("sisam"), "sisam"); // → シサㇺ
        assert_eq!(normalize("eyaykosiramsuypa"), "eyaykosiramsuypa"); // → エヤイコシラㇺスイパ
    }

    // ---- Edge cases / robustness. ----

    #[test]
    fn empty_and_trivial() {
        assert_eq!(normalize(""), "");
        assert_eq!(normalize("a"), "a");
        assert_eq!(normalize("t"), "t");
        assert_eq!(normalize("c"), "c");
        assert_eq!(normalize("ch"), "c"); // trailing ch with no vowel
        assert_eq!(normalize("f"), "f"); // bare f untouched (no `fu`)
    }

    #[test]
    fn combined_legacy_input() {
        // A maximally legacy spelling exercising several rules at once.
        // `Fuchi` → lowercase `fuchi` → R8 `fu`→`hu` → R3 `chi`→`ci` = `huci`.
        assert_eq!(normalize("Fuchi"), "huci"); // → フチ
                                                // `Chishi` → `cishi`? No: `chi`→`ci`, then `shi`→`si` (onset after `i`).
        assert_eq!(normalize("Chishi"), "cisi");
    }
}
