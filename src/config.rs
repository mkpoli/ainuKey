//! Two-way TOML configuration.
//!
//! The IME reads its behavior from a [`Config`], persisted as TOML at
//! `%APPDATA%\ainuKey\config.toml`. The file is the source of truth: it can be
//! edited by hand **or** through the settings GUI, and both stay in sync because
//! each goes through this module (load on activation / on file change; save from
//! the GUI). Every field has a default and `#[serde(default)]`, so an old,
//! partial, or hand-trimmed file still loads, and a malformed file falls back to
//! defaults rather than breaking input.
//!
//! Pure `std` + `serde` (no `windows`), so it unit-tests on any host. Lands
//! ahead of its consumers (the settings GUI and the conversion/input paths).
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// How to render the `tu` mora in katakana. Ainu `tu` has several attested
/// spellings; pick the one your materials use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TuStyle {
    /// `ainconv`'s default: ト゚ (ト with a combining handakuten).
    #[default]
    To,
    /// ツ゚ — ツ with a combining handakuten.
    Tsu,
    /// トゥ — ト followed by a small ゥ.
    Twu,
    /// ツ — a plain katakana tsu, no handakuten.
    PlainTsu,
}

/// Mode a fresh composition starts in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InputMode {
    /// Convert romaji to katakana (normal IME mode).
    #[default]
    Kana,
    /// Pass Latin through untouched.
    Latin,
}

/// Input behaviour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Input {
    /// Mode a fresh composition starts in.
    pub default_mode: InputMode,
    /// Show the romaji while typing and defer katakana conversion to commit.
    pub romaji_input_mode: bool,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            default_mode: InputMode::Kana,
            romaji_input_mode: false,
        }
    }
}

/// Katakana notation options, applied on top of the default `ainconv`
/// conversion. Mirrors the canonical option set in `ainconv-tests`
/// (`options.schema.json`): `useWi`/`useWe`/`useWo`, `useSmallI`/`U`/`N`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Orthography {
    /// How to render the `tu` mora (ト゚ / ツ゚ / トゥ / ツ).
    pub tu_style: TuStyle,
    /// Render the `-y` coda as a small ィ instead of イ.
    pub use_small_i: bool,
    /// Render the `-w` coda as a small ゥ instead of ウ.
    pub use_small_u: bool,
    /// Render the `-n` coda as a small ㇴ instead of ン.
    pub use_small_n: bool,
    /// Render onset `wi` as ヰ instead of ウィ.
    pub use_wi: bool,
    /// Render onset `we` as ヱ instead of ウェ.
    pub use_we: bool,
    /// Render onset `wo` as ヲ instead of ウォ.
    pub use_wo: bool,
    /// Show the `=` morpheme boundary in the output (`ainconv` strips it).
    pub show_equals_boundary: bool,
}

impl Default for Orthography {
    fn default() -> Self {
        Self {
            tu_style: TuStyle::To,
            use_small_i: false,
            use_small_u: false,
            use_small_n: false,
            use_wi: false,
            use_we: false,
            use_wo: false,
            show_equals_boundary: false,
        }
    }
}

/// Suggestion / candidate-window options.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Suggestions {
    /// Show the candidate window with n-gram suggestions.
    pub enabled: bool,
    /// Maximum candidates in the list.
    pub max_candidates: usize,
    /// Re-rank completions by trigram/bigram context.
    pub context_aware: bool,
}

impl Default for Suggestions {
    fn default() -> Self {
        Self {
            enabled: true,
            max_candidates: 9,
            context_aware: true,
        }
    }
}

/// The full configuration.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub input: Input,
    pub orthography: Orthography,
    pub suggestions: Suggestions,
}

impl Config {
    /// `%APPDATA%\ainuKey\config.toml`, or `None` if `%APPDATA%` is unset.
    pub fn path() -> Option<PathBuf> {
        let appdata = std::env::var_os("APPDATA")?;
        Some(PathBuf::from(appdata).join("ainuKey").join("config.toml"))
    }

    /// Load from `%APPDATA%`, falling back to defaults for a missing or malformed
    /// file (input must never break because the config is bad).
    pub fn load() -> Self {
        match Self::path() {
            Some(p) => Self::load_from(&p),
            None => Self::default(),
        }
    }

    /// Load from a specific path (no `%APPDATA%` needed — testable).
    pub fn load_from(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text),
            Err(_) => Self::default(),
        }
    }

    /// Parse from TOML text; defaults on any parse error.
    pub fn parse(text: &str) -> Self {
        toml::from_str(text).unwrap_or_default()
    }

    /// Serialize to pretty TOML.
    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_default()
    }

    /// Serialize to TOML **with inline documentation** — a header plus, for every
    /// option, a comment describing it and listing its allowed values. This is
    /// what gets written to disk, so the file is tunable by hand without guessing.
    /// Comments are ignored by [`parse`](Self::parse), so it still round-trips.
    pub fn to_documented_toml(&self) -> String {
        let i = &self.input;
        let o = &self.orthography;
        let s = &self.suggestions;
        let mode = match i.default_mode {
            InputMode::Kana => "kana",
            InputMode::Latin => "latin",
        };
        let tu = match o.tu_style {
            TuStyle::To => "to",
            TuStyle::Tsu => "tsu",
            TuStyle::Twu => "twu",
            TuStyle::PlainTsu => "plain_tsu",
        };
        format!(
            "\
# ainuKey configuration  —  %APPDATA%\\ainuKey\\config.toml
#
# Edit and save, then re-activate ainuKey (switch input away and back, or restart
# the app) to apply. Lines beginning with '#' are comments. Each option lists its
# allowed values; delete this file to regenerate it with the defaults below.

[input]
# Mode a new composition starts in.
#   kana  = convert romaji to Ainu katakana (normal)
#   latin = pass Latin letters through unchanged
default_mode = \"{mode}\"
# Show romaji while typing and convert to katakana only on commit.  (true | false)
romaji_input_mode = {romaji}

[orthography]
# How to render the `tu` mora:
#   to        = ト゚   (ト + handakuten; ainconv default)
#   tsu       = ツ゚   (ツ + handakuten)
#   twu       = トゥ
#   plain_tsu = ツ    (plain katakana tsu)
tu_style = \"{tu}\"
# Render the -y coda as a small ィ instead of イ.  (true | false)
use_small_i = {usi}
# Render the -w coda as a small ゥ instead of ウ.  (true | false)
use_small_u = {usu}
# Render the -n coda as a small ㇴ instead of ン.  (true | false)
use_small_n = {usn}
# Render onset `wi` as ヰ instead of ウィ.  (true | false)
use_wi = {uwi}
# Render onset `we` as ヱ instead of ウェ.  (true | false)
use_we = {uwe}
# Render onset `wo` as ヲ instead of ウォ.  (true | false)
use_wo = {uwo}
# Keep the `=` morpheme boundary in the output (ainconv strips it).  (true | false)
show_equals_boundary = {eq}

[suggestions]
# Show the candidate window with word suggestions.  (true | false)
enabled = {en}
# Maximum number of candidates shown in the list.  (integer, e.g. 9)
max_candidates = {mc}
# Re-rank completions by trigram/bigram context.  (true | false)
context_aware = {ca}
",
            mode = mode,
            romaji = i.romaji_input_mode,
            tu = tu,
            usi = o.use_small_i,
            usu = o.use_small_u,
            usn = o.use_small_n,
            uwi = o.use_wi,
            uwe = o.use_we,
            uwo = o.use_wo,
            eq = o.show_equals_boundary,
            en = s.enabled,
            mc = s.max_candidates,
            ca = s.context_aware,
        )
    }

    /// Atomically write to `%APPDATA%\ainuKey\config.toml` (no-op if unset).
    pub fn save(&self) -> std::io::Result<()> {
        match Self::path() {
            Some(p) => self.save_to(&p),
            None => Ok(()),
        }
    }

    /// Write to a specific path: create the directory, write to a temp file, then
    /// rename, so a reader never sees a half-written file.
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("toml.tmp");
        // Write the self-documenting form so a hand-editor sees what each option
        // means and its allowed values.
        std::fs::write(&tmp, self.to_documented_toml())?;
        std::fs::rename(&tmp, path)
    }
}

use std::sync::RwLock;

/// Process-wide cached config. `None` until first use / a `reload`.
static GLOBAL: RwLock<Option<Config>> = RwLock::new(None);

/// The current config, loaded from disk lazily and cached. Cheap to call (clones
/// a small struct); used on the hot input path.
pub fn current() -> Config {
    if let Ok(g) = GLOBAL.read() {
        if let Some(c) = g.as_ref() {
            return c.clone();
        }
    }
    reload()
}

/// Re-read the config from disk and refresh the cache. Call at activation and
/// after the settings GUI writes the file.
pub fn reload() -> Config {
    let c = Config::load();
    if let Ok(mut g) = GLOBAL.write() {
        *g = Some(c.clone());
    }
    c
}

/// Write the default config file if it doesn't exist yet, so it is discoverable
/// and hand-editable even before the settings GUI is ever opened.
pub fn ensure_file() {
    if let Some(p) = Config::path() {
        if !p.exists() {
            let _ = Config::default().save_to(&p);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sensible() {
        let c = Config::default();
        assert_eq!(c.input.default_mode, InputMode::Kana);
        assert!(!c.input.romaji_input_mode);
        assert_eq!(c.orthography.tu_style, TuStyle::To);
        assert!(c.suggestions.enabled);
        assert_eq!(c.suggestions.max_candidates, 9);
    }

    #[test]
    fn toml_round_trips() {
        let mut c = Config::default();
        c.orthography.tu_style = TuStyle::Twu;
        c.orthography.use_small_i = true;
        c.input.default_mode = InputMode::Latin;
        c.suggestions.max_candidates = 5;
        assert_eq!(Config::parse(&c.to_toml()), c);
    }

    #[test]
    fn documented_toml_round_trips_and_covers_every_field() {
        // Every field set to a NON-default value: if to_documented_toml omits any
        // field, parse() returns its default and this comparison fails — so the
        // documented template can't silently drift out of sync with the struct.
        let c = Config {
            input: Input {
                default_mode: InputMode::Latin,
                romaji_input_mode: true,
            },
            orthography: Orthography {
                tu_style: TuStyle::PlainTsu,
                use_small_i: true,
                use_small_u: true,
                use_small_n: true,
                use_wi: true,
                use_we: true,
                use_wo: true,
                show_equals_boundary: true,
            },
            suggestions: Suggestions {
                enabled: false,
                max_candidates: 4,
                context_aware: false,
            },
        };
        let doc = c.to_documented_toml();
        assert_eq!(Config::parse(&doc), c, "documented TOML must round-trip");
        // It really is documented: comments + an allowed-values list are present.
        assert!(doc.contains("# ainuKey configuration"));
        assert!(doc.contains("plain_tsu = ツ"), "tu_style values listed: {doc}");
        assert!(doc.contains("(true | false)"));
    }

    #[test]
    fn empty_text_is_defaults() {
        assert_eq!(Config::parse(""), Config::default());
    }

    #[test]
    fn partial_file_fills_missing_with_defaults() {
        // Only one field of one section given; everything else defaults.
        let c = Config::parse("[orthography]\nuse_small_i = true\n");
        assert!(c.orthography.use_small_i);
        assert!(!c.orthography.use_small_u); // not given → defaulted
        assert_eq!(c.orthography.tu_style, TuStyle::To); // defaulted
        assert_eq!(c.suggestions.max_candidates, 9); // whole section defaulted
        assert_eq!(c.input.default_mode, InputMode::Kana);
    }

    #[test]
    fn malformed_toml_falls_back_to_defaults() {
        assert_eq!(
            Config::parse("this is not = = valid toml [["),
            Config::default()
        );
        // wrong type for a field → still defaults, never panics
        assert_eq!(
            Config::parse("[suggestions]\nmax_candidates = \"lots\"\n"),
            Config::default()
        );
    }

    #[test]
    fn unknown_fields_are_ignored() {
        // Forward-compat: a field this binary doesn't know is skipped, not fatal.
        let c = Config::parse("[input]\nfuture_option = 42\ndefault_mode = \"latin\"\n");
        assert_eq!(c.input.default_mode, InputMode::Latin);
    }

    #[test]
    fn enums_serialize_snake_case() {
        let t = Config::default().to_toml();
        assert!(t.contains("default_mode = \"kana\""), "{t}");
        assert!(t.contains("tu_style = \"to\""), "{t}");
    }

    #[test]
    fn save_then_load_from_disk() {
        let dir = std::env::temp_dir().join(format!("ainukey-cfg-test-{}", std::process::id()));
        let path = dir.join("config.toml");
        let mut c = Config::default();
        c.suggestions.enabled = false;
        c.input.romaji_input_mode = true;
        c.save_to(&path).expect("save");
        assert_eq!(Config::load_from(&path), c);
        // missing file → defaults
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(Config::load_from(&path), Config::default());
    }
}
