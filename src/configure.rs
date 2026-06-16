//! Settings entry point: `ITfFunctionProvider` + `ITfFnConfigure` so the
//! "Options" button next to ainuKey in the Windows language settings opens a
//! configuration dialog. v1 shows a simple about/help box; a richer dialog is a
//! follow-up.
//!
//! The system discovers this by QI-ing the activated TIP object (our
//! `TextService`, which now also implements these three interfaces) and calling
//! `GetFunction(_, IID_ITfFnConfigure)` → `ITfFnConfigure::Show`.

use windows::core::{w, IUnknown, IUnknownImpl, Interface, BSTR, GUID};
use windows::Win32::Foundation::{E_NOINTERFACE, HWND};
use windows::Win32::UI::TextServices::{
    ITfFnConfigure, ITfFnConfigure_Impl, ITfFunctionProvider_Impl, ITfFunction_Impl,
};
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONINFORMATION, MB_OK};

use crate::guids::GUID_TEXT_SERVICE;
use crate::text_service::TextService_Impl;

impl ITfFunction_Impl for TextService_Impl {
    fn GetDisplayName(&self) -> windows::core::Result<BSTR> {
        Ok(BSTR::from("ainuKey Configuration"))
    }
}

impl ITfFunctionProvider_Impl for TextService_Impl {
    fn GetType(&self) -> windows::core::Result<GUID> {
        Ok(GUID_TEXT_SERVICE)
    }

    fn GetDescription(&self) -> windows::core::Result<BSTR> {
        Ok(BSTR::from("ainuKey"))
    }

    fn GetFunction(
        &self,
        _rguid: *const GUID,
        riid: *const GUID,
    ) -> windows::core::Result<IUnknown> {
        // SAFETY: riid is a valid GUID pointer supplied by the caller.
        if unsafe { riid.as_ref() } == Some(&ITfFnConfigure::IID) {
            let cfg: ITfFnConfigure = self.to_interface();
            Ok(cfg.cast()?)
        } else {
            Err(E_NOINTERFACE.into())
        }
    }
}

impl ITfFnConfigure_Impl for TextService_Impl {
    fn Show(
        &self,
        hwndparent: HWND,
        _langid: u16,
        _rguidprofile: *const GUID,
    ) -> windows::core::Result<()> {
        // SAFETY: hwndparent may be null (a valid "no owner"); the strings are
        // static NUL-terminated literals.
        unsafe {
            // Trilingual (Ainu name + 日本語 + English). Ainu terms are from the
            // itak.aynu.org glossary: aeynuyep = IME (< まぽ), iyanu = settings.
            MessageBoxW(
                Some(hwndparent),
                w!("ainuKey — Aynu itak aeynuyep\r\n\
                    アイヌ語入力（IME） / Ainu-language IME\r\n\r\n\
                    ・ローマ字を入力し、Space/Enter でカタカナに変換します。\r\n\
                    Type romaji; Space/Enter converts to Ainu katakana.\r\n\
                    ・ti → ci のような寛容な入力 / forgiving input (ti → ci).\r\n\
                    ・候補: ↑/↓ で選択、1–9 で確定 / suggestions: ↑/↓, 1–9.\r\n\
                    ・言語バーでカタカナ／ローマ字を切替 / toggle katakana / Latin.\r\n\r\n\
                    iyanu / 設定 / settings — more coming soon."),
                w!("ainuKey — iyanu（設定 / settings）"),
                MB_OK | MB_ICONINFORMATION,
            );
        }
        Ok(())
    }
}
