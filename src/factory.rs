//! The class factory. A distinct, tiny COM object (not the `TextService`)
//! because `DllGetClassObject` creates the factory before any `TextService`
//! exists.

use std::ffi::c_void;

use windows::core::{implement, IUnknown, Interface, Ref, GUID};
use windows::Win32::Foundation::{CLASS_E_NOAGGREGATION, E_POINTER};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};
use windows::Win32::UI::TextServices::ITfTextInputProcessor;

use crate::text_service::TextService;
use crate::{lock_module, unlock_module};

#[implement(IClassFactory)]
pub struct ClassFactory;

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(
        &self,
        punkouter: Ref<'_, IUnknown>,
        riid: *const GUID,
        ppvobject: *mut *mut c_void,
    ) -> windows::core::Result<()> {
        if ppvobject.is_null() {
            return Err(E_POINTER.into());
        }
        // SAFETY: caller contract — ppvobject is a valid, writable location.
        unsafe {
            *ppvobject = std::ptr::null_mut();
        }
        if !punkouter.is_null() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }

        // Build the text service and hand back the requested interface. The
        // object implements ITfTextInputProcessor(Ex); the macro-generated
        // QueryInterface answers every implemented IID. `query` AddRefs the
        // returned pointer; the local `tip` handle drops -> Release; net = 1.
        let tip: ITfTextInputProcessor = TextService::new().into();
        // SAFETY: ppvobject validated above; riid is a valid GUID pointer.
        unsafe { tip.query(riid, ppvobject).ok() }
    }

    fn LockServer(&self, flock: windows::core::BOOL) -> windows::core::Result<()> {
        if flock.as_bool() {
            lock_module();
        } else {
            unlock_module();
        }
        Ok(())
    }
}
