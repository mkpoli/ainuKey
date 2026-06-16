//! Display-attribute provider + the single underline display-attribute info
//! object and its enumerator. v1 ships exactly one display attribute: a dotted
//! underline used to style the preedit.

use std::cell::Cell;

use windows::core::{implement, OutRef, BSTR, GUID};
use windows::Win32::UI::TextServices::{
    IEnumTfDisplayAttributeInfo, IEnumTfDisplayAttributeInfo_Impl, ITfDisplayAttributeInfo,
    ITfDisplayAttributeInfo_Impl, ITfDisplayAttributeProvider_Impl, TF_ATTR_INPUT, TF_CT_NONE,
    TF_DA_COLOR, TF_DISPLAYATTRIBUTE, TF_LS_DOT,
};

use crate::guids::GUID_DISPLAY_ATTRIBUTE;
use crate::text_service::TextService_Impl;

impl ITfDisplayAttributeProvider_Impl for TextService_Impl {
    fn EnumDisplayAttributeInfo(&self) -> windows::core::Result<IEnumTfDisplayAttributeInfo> {
        Ok(EnumDisplayAttributeInfo::new().into())
    }

    fn GetDisplayAttributeInfo(
        &self,
        guid: *const GUID,
    ) -> windows::core::Result<ITfDisplayAttributeInfo> {
        // SAFETY: TSF passes a valid GUID pointer.
        let requested = unsafe { *guid };
        if requested == GUID_DISPLAY_ATTRIBUTE {
            Ok(DisplayAttributeInfo.into())
        } else {
            Err(windows::Win32::Foundation::E_INVALIDARG.into())
        }
    }
}

/// The single underline display-attribute info object.
#[implement(ITfDisplayAttributeInfo)]
pub struct DisplayAttributeInfo;

impl DisplayAttributeInfo {
    fn attribute() -> TF_DISPLAYATTRIBUTE {
        TF_DISPLAYATTRIBUTE {
            crText: TF_DA_COLOR {
                r#type: TF_CT_NONE,
                ..Default::default()
            },
            crBk: TF_DA_COLOR {
                r#type: TF_CT_NONE,
                ..Default::default()
            },
            lsStyle: TF_LS_DOT,
            fBoldLine: false.into(),
            crLine: TF_DA_COLOR {
                r#type: TF_CT_NONE,
                ..Default::default()
            },
            bAttr: TF_ATTR_INPUT,
        }
    }
}

impl ITfDisplayAttributeInfo_Impl for DisplayAttributeInfo_Impl {
    fn GetGUID(&self) -> windows::core::Result<GUID> {
        Ok(GUID_DISPLAY_ATTRIBUTE)
    }

    fn GetDescription(&self) -> windows::core::Result<BSTR> {
        Ok(BSTR::from("ainuKey preedit"))
    }

    fn GetAttributeInfo(&self, pda: *mut TF_DISPLAYATTRIBUTE) -> windows::core::Result<()> {
        if pda.is_null() {
            return Err(windows::Win32::Foundation::E_POINTER.into());
        }
        // SAFETY: pda validated non-null; TSF provides a writable location.
        unsafe {
            *pda = DisplayAttributeInfo::attribute();
        }
        Ok(())
    }

    fn SetAttributeInfo(&self, _pda: *const TF_DISPLAYATTRIBUTE) -> windows::core::Result<()> {
        // Immutable.
        Ok(())
    }

    fn Reset(&self) -> windows::core::Result<()> {
        Ok(())
    }
}

/// Enumerator yielding the single display-attribute info, then ending.
#[implement(IEnumTfDisplayAttributeInfo)]
pub struct EnumDisplayAttributeInfo {
    /// `false` until the single item has been yielded.
    done: Cell<bool>,
}

impl EnumDisplayAttributeInfo {
    pub fn new() -> Self {
        Self {
            done: Cell::new(false),
        }
    }
}

impl IEnumTfDisplayAttributeInfo_Impl for EnumDisplayAttributeInfo_Impl {
    fn Clone(&self) -> windows::core::Result<IEnumTfDisplayAttributeInfo> {
        let fresh = EnumDisplayAttributeInfo {
            done: Cell::new(self.done.get()),
        };
        Ok(fresh.into())
    }

    fn Next(
        &self,
        ulcount: u32,
        rginfo: OutRef<'_, ITfDisplayAttributeInfo>,
        pcfetched: *mut u32,
    ) -> windows::core::Result<()> {
        let mut fetched: u32 = 0;
        if ulcount >= 1 && !self.done.get() {
            self.done.set(true);
            let info: ITfDisplayAttributeInfo = DisplayAttributeInfo.into();
            rginfo.write(Some(info))?;
            fetched = 1;
        }
        if !pcfetched.is_null() {
            // SAFETY: caller-provided writable location when non-null.
            unsafe {
                *pcfetched = fetched;
            }
        }
        // S_OK when we fetched the requested count, S_FALSE otherwise.
        if fetched == ulcount {
            Ok(())
        } else {
            Err(windows::Win32::Foundation::S_FALSE.into())
        }
    }

    fn Reset(&self) -> windows::core::Result<()> {
        self.done.set(false);
        Ok(())
    }

    fn Skip(&self, ulcount: u32) -> windows::core::Result<()> {
        if ulcount == 0 {
            return Ok(());
        }
        // Exactly one element exists. If it's already consumed, or more than one
        // was requested, we cannot skip the full count -> S_FALSE.
        let already_done = self.done.get();
        self.done.set(true);
        if already_done || ulcount > 1 {
            Err(windows::Win32::Foundation::S_FALSE.into())
        } else {
            Ok(())
        }
    }
}
