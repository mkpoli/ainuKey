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
use windows::Win32::UI::TextServices::{
    ITfLangBarItem, ITfLangBarItemButton, ITfLangBarItemButton_Impl, ITfLangBarItemSink,
    ITfLangBarItem_Impl, ITfMenu, ITfSource, ITfSource_Impl, TfLBIClick, TF_LANGBARITEMINFO,
    TF_LBI_ICON, TF_LBI_STYLE_BTN_BUTTON, TF_LBI_TEXT, TF_LBI_TOOLTIP,
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
                dwStyle: TF_LBI_STYLE_BTN_BUTTON,
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
        Ok(BSTR::from(match self.mode.get() {
            Mode::Kana => "ainuKey — katakana mode (click for Latin)",
            Mode::Latin => "ainuKey — Latin mode (click for katakana)",
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
        let next = match self.mode.get() {
            Mode::Kana => Mode::Latin,
            Mode::Latin => Mode::Kana,
        };
        self.mode.set(next);
        if let Some(sink) = self.sink.borrow().as_ref() {
            // SAFETY: a valid sink advised via AdviseSink.
            unsafe {
                let _ = sink.OnUpdate(TF_LBI_ICON | TF_LBI_TEXT | TF_LBI_TOOLTIP);
            }
        }
        Ok(())
    }

    fn InitMenu(&self, _pmenu: Ref<'_, ITfMenu>) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnMenuSelect(&self, _wid: u32) -> windows::core::Result<()> {
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
