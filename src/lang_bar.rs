//! Language-bar mode button: toggles between **Kana** (romaji → Ainu katakana,
//! the default) and **Latin** (direct alphabet passthrough) input.
//!
//! A single COM object implements the three TSF langbar interfaces
//! (`ITfLangBarItemButton` + its base `ITfLangBarItem`, and `ITfSource` so the
//! shell can advise an update sink). It shares the input [`Mode`] with the text
//! service via an `Rc<Cell<Mode>>`, so `OnClick` flips the mode the key handler
//! reads.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::core::{implement, IUnknown, Interface, Ref, BSTR, GUID, PCWSTR};
use windows::Win32::Foundation::{E_INVALIDARG, E_NOINTERFACE, HINSTANCE, POINT, RECT};
use windows::Win32::Graphics::Gdi::HBITMAP;
use windows::Win32::UI::TextServices::{
    ITfLangBarItem, ITfLangBarItemButton, ITfLangBarItemButton_Impl, ITfLangBarItemSink,
    ITfLangBarItem_Impl, ITfMenu, ITfSource, ITfSource_Impl, TfLBIClick, TF_LANGBARITEMINFO,
    TF_LBI_ICON, TF_LBI_STYLE_BTN_MENU, TF_LBI_TEXT, TF_LBI_TOOLTIP,
};
use windows::Win32::UI::WindowsAndMessaging::{LoadIconW, HICON};

use crate::dll_instance;
use crate::guids::{GUID_LANGBAR_ITEM, GUID_TEXT_SERVICE, IDI_MODE_KANA, IDI_MODE_LATN};

/// Input mode toggled by the language-bar button.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mode {
    /// Romaji is converted to Ainu katakana (the default).
    #[default]
    Kana,
    /// Latin letters pass through unchanged (direct alphabet input).
    Latin,
}

// Drop-down menu item IDs and (TSF) menu-item flags.
const MENU_KANA: u32 = 1;
const MENU_LATIN: u32 = 2;
const MENU_SETTINGS: u32 = 100;
const TF_LBMENUF_SEPARATOR: u32 = 0x0000_0004;
const TF_LBMENUF_RADIOCHECKED: u32 = 0x0000_0008;

/// Add one item to a langbar drop-down menu.
///
/// # Safety
/// `menu` must be a valid `ITfMenu` (supplied by the framework in `InitMenu`).
unsafe fn add_menu_item(
    menu: &ITfMenu,
    id: u32,
    flags: u32,
    text: &str,
) -> windows::core::Result<()> {
    let wide: Vec<u16> = text.encode_utf16().collect();
    menu.AddMenuItem(
        id,
        flags,
        HBITMAP::default(),
        HBITMAP::default(),
        &wide,
        std::ptr::null_mut(),
    )
}

/// The language-bar toggle button.
#[implement(ITfLangBarItemButton, ITfLangBarItem, ITfSource)]
pub struct ModeButton {
    /// Shared with the text service; `OnClick` flips it.
    mode: Rc<Cell<Mode>>,
    /// The langbar's advised sink, for `OnUpdate` notifications after a toggle.
    sink: RefCell<Option<ITfLangBarItemSink>>,
}

impl ModeButton {
    pub fn new(mode: Rc<Cell<Mode>>) -> Self {
        Self {
            mode,
            sink: RefCell::new(None),
        }
    }
}

impl ModeButton_Impl {
    fn icon_id(&self) -> u16 {
        match self.mode.get() {
            Mode::Kana => IDI_MODE_KANA,
            Mode::Latin => IDI_MODE_LATN,
        }
    }

    /// Set the mode and notify the langbar sink to repaint the icon/text/tooltip.
    fn set_mode(&self, mode: Mode) {
        self.mode.set(mode);
        if let Some(sink) = self.sink.borrow().as_ref() {
            // SAFETY: a valid sink advised via AdviseSink.
            unsafe {
                let _ = sink.OnUpdate(TF_LBI_ICON | TF_LBI_TEXT | TF_LBI_TOOLTIP);
            }
        }
    }
}

impl ITfLangBarItem_Impl for ModeButton_Impl {
    fn GetInfo(&self, pinfo: *mut TF_LANGBARITEMINFO) -> windows::core::Result<()> {
        if pinfo.is_null() {
            return Err(E_INVALIDARG.into());
        }
        let mut desc = [0u16; 32];
        for (slot, c) in desc.iter_mut().zip("ainuKey".encode_utf16()) {
            *slot = c;
        }
        // SAFETY: pinfo checked non-null; we fully overwrite the struct.
        unsafe {
            *pinfo = TF_LANGBARITEMINFO {
                clsidService: GUID_TEXT_SERVICE,
                guidItem: GUID_LANGBAR_ITEM,
                dwStyle: TF_LBI_STYLE_BTN_MENU,
                ulSort: 0,
                szDescription: desc,
            };
        }
        Ok(())
    }

    fn GetStatus(&self) -> windows::core::Result<u32> {
        Ok(0) // visible + enabled
    }

    fn Show(&self, _fshow: windows::core::BOOL) -> windows::core::Result<()> {
        Ok(())
    }

    fn GetTooltipString(&self) -> windows::core::Result<BSTR> {
        // ainuKey = "Aynu itak aeynuyep" (Ainu-language IME; glossary: aeynuyep).
        // katakana/Latin have no coined Ainu term, so the script names are kept;
        // the message is trilingual (Ainu name + 日本語 + English).
        Ok(BSTR::from(match self.mode.get() {
            Mode::Kana => "ainuKey — カタカナ katakana（クリックでローマ字 / click for Latin）",
            Mode::Latin => "ainuKey — ローマ字 Latin（クリックでカタカナ / click for katakana）",
        }))
    }
}

impl ITfLangBarItemButton_Impl for ModeButton_Impl {
    fn OnClick(
        &self,
        _click: TfLBIClick,
        _pt: &POINT,
        _prcarea: *const RECT,
    ) -> windows::core::Result<()> {
        // A bare click (when the framework reports one) still toggles the mode.
        let next = match self.mode.get() {
            Mode::Kana => Mode::Latin,
            Mode::Latin => Mode::Kana,
        };
        self.set_mode(next);
        Ok(())
    }

    fn InitMenu(&self, pmenu: Ref<'_, ITfMenu>) -> windows::core::Result<()> {
        let menu = pmenu.ok()?;
        let (kana, latin) = match self.mode.get() {
            Mode::Kana => (TF_LBMENUF_RADIOCHECKED, 0),
            Mode::Latin => (0, TF_LBMENUF_RADIOCHECKED),
        };
        // SAFETY: `menu` is a valid ITfMenu provided by the framework.
        unsafe {
            add_menu_item(menu, MENU_KANA, kana, "カタカナ Katakana")?;
            add_menu_item(menu, MENU_LATIN, latin, "ローマ字 Latin")?;
            add_menu_item(menu, 0, TF_LBMENUF_SEPARATOR, "")?;
            add_menu_item(menu, MENU_SETTINGS, 0, "設定 / Settings…")?;
        }
        Ok(())
    }

    fn OnMenuSelect(&self, wid: u32) -> windows::core::Result<()> {
        match wid {
            MENU_KANA => self.set_mode(Mode::Kana),
            MENU_LATIN => self.set_mode(Mode::Latin),
            // The reachable entry point for the settings dialog on Windows 11,
            // where the language-settings "Options" is not exposed for TIPs.
            MENU_SETTINGS => {
                crate::settings_dialog::show(windows::Win32::Foundation::HWND::default())
            }
            _ => {}
        }
        Ok(())
    }

    fn GetIcon(&self) -> windows::core::Result<HICON> {
        let hinst = HINSTANCE(dll_instance().0);
        // SAFETY: hinst is this DLL's module; the id is a valid icon resource.
        unsafe { LoadIconW(Some(hinst), PCWSTR(self.icon_id() as usize as *const u16)) }
    }

    fn GetText(&self) -> windows::core::Result<BSTR> {
        Ok(BSTR::from(match self.mode.get() {
            Mode::Kana => "ア",
            Mode::Latin => "A",
        }))
    }
}

impl ITfSource_Impl for ModeButton_Impl {
    fn AdviseSink(&self, riid: *const GUID, punk: Ref<'_, IUnknown>) -> windows::core::Result<u32> {
        // SAFETY: riid is a valid GUID pointer from the caller.
        if unsafe { riid.as_ref() } != Some(&ITfLangBarItemSink::IID) {
            return Err(E_NOINTERFACE.into());
        }
        let sink: ITfLangBarItemSink = punk.ok()?.cast()?;
        *self.sink.borrow_mut() = Some(sink);
        Ok(1) // single sink; fixed cookie
    }

    fn UnadviseSink(&self, _dwcookie: u32) -> windows::core::Result<()> {
        *self.sink.borrow_mut() = None;
        Ok(())
    }
}
