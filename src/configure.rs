//! Settings entry point: `ITfFunctionProvider` + `ITfFnConfigure` so the
//! "Options" button next to ainuKey in the Windows language settings opens a
//! configuration dialog. v1 shows a simple about/help box; a richer dialog is a
//! follow-up.
//!
//! The system discovers this by QI-ing the activated TIP object (our
//! `TextService`, which now also implements these three interfaces) and calling
//! `GetFunction(_, IID_ITfFnConfigure)` → `ITfFnConfigure::Show`.

use windows::core::{IUnknown, IUnknownImpl, Interface, BSTR, GUID};
use windows::Win32::Foundation::{E_NOINTERFACE, HWND};
use windows::Win32::UI::TextServices::{
    ITfFnConfigure, ITfFnConfigure_Impl, ITfFunctionProvider_Impl, ITfFunction_Impl,
};

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
        // Open the native settings dialog (checkboxes bound to config.toml).
        crate::settings_dialog::show(hwndparent);
        Ok(())
    }
}
