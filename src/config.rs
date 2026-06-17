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

/// How to render the `tu` mora / `t` onset in katakana.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TuStyle {
    /// `ainconv`'s convention: ト゚.
    #[default]
    To,
    /// Alternative convention: ツ゚.
    Tsu,
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

/// Katakana notation options (post-processed over `ainconv`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Orthography {
    /// Rendering of the `tu`/`t` sound.
    pub tu_style: TuStyle,
    /// Keep the small glide codas (`y`→ィ, `w`→ゥ) instead of collapsing to イ/ウ.
    pub small_glides: bool,
    /// Show the `=` morpheme boundary in the output (`ainconv` strips it).
    pub show_equals_boundary: bool,
}

impl Default for Orthography {
    fn default() -> Self {
        Self {
            tu_style: TuStyle::To,
            small_glides: false,
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
        std::fs::write(&tmp, self.to_toml())?;
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
        c.orthography.tu_style = TuStyle::Tsu;
        c.orthography.small_glides = true;
        c.input.default_mode = InputMode::Latin;
        c.suggestions.max_candidates = 5;
        assert_eq!(Config::parse(&c.to_toml()), c);
    }

    #[test]
    fn empty_text_is_defaults() {
        assert_eq!(Config::parse(""), Config::default());
    }

    #[test]
    fn partial_file_fills_missing_with_defaults() {
        // Only one field of one section given; everything else defaults.
        let c = Config::parse("[orthography]\nsmall_glides = true\n");
        assert!(c.orthography.small_glides);
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
