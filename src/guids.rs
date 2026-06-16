//! Hardcoded GUIDs / names / locale constants for ainuKey.
//!
//! From IDENTITY (authoritative). The two service GUIDs differ only in the last
//! hex digit of the third group (`…CB` vs `…CC`).

use windows::core::{w, GUID, PCWSTR};

/// Text-service CLSID: {5ECECCEB-271D-4675-8EE5-8D129EF0CA08}
pub const GUID_TEXT_SERVICE: GUID = GUID::from_u128(0x5ECECCEB_271D_4675_8EE5_8D129EF0CA08);

/// Language profile GUID: {5ECECCEC-271D-4675-8EE5-8D129EF0CA08}
pub const GUID_PROFILE: GUID = GUID::from_u128(0x5ECECCEC_271D_4675_8EE5_8D129EF0CA08);

/// Our private display-attribute GUID (the underline style for the preedit).
/// A dedicated v4-style UUID distinct from the service GUIDs.
pub const GUID_DISPLAY_ATTRIBUTE: GUID = GUID::from_u128(0x5ECECCED_271D_4675_8EE5_8D129EF0CA08);

/// GUID identifying our language-bar (mode-switch) item.
pub const GUID_LANGBAR_ITEM: GUID = GUID::from_u128(0x5ECECCEF_271D_4675_8EE5_8D129EF0CA08);

/// Icon resource IDs (see `resources.rc`). The kana/latin mode icons drive the
/// language-bar button; `IDI_ICON` (101) is the profile icon at index 0.
pub const IDI_MODE_KANA: u16 = 102;
pub const IDI_MODE_LATN: u16 = 103;

/// Registration / service name (subkey default value, profile name fallback).
pub const SERVICE_NAME: PCWSTR = w!("ainuKeyTextService");

/// Human-facing display description shown in the language switcher.
pub const DISPLAY_DESCRIPTION: PCWSTR = w!("ainuKey");

/// BCP-47 language tag for Ainu. (Kept for identity; the profile is registered
/// under the ja-JP langid in v1, so this is not yet referenced.)
#[allow(dead_code)]
pub const LANG_TAG_AIN: PCWSTR = w!("ain");

/// Locale used ONLY to obtain a concrete numeric langid for RegisterProfile,
/// because Ainu has no assigned Windows LCID. We register the profile under the
/// Japanese locale's langid so it appears in the JP input-method group.
pub const PROFILE_LOCALE: PCWSTR = w!("ja-JP");

/// LOCALE_CUSTOM_UNSPECIFIED = MAKELANGID(LANG_NEUTRAL, SUBLANG_CUSTOM_UNSPECIFIED).
/// Retained as the canonical "Ainu has no LCID" constant; used if/when a
/// custom-locale path is preferred over ja-JP. (Tracked follow-up, not v1.)
#[allow(dead_code)]
pub const LOCALE_CUSTOM_UNSPECIFIED: u16 = 0x1000;
