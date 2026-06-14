//! ainuKey — a minimal, deterministic Ainu Text Services Framework (TSF) Text
//! Input Processor (TIP) for Windows, written in Rust.
//!
//! Crate root: module declarations + the five DLL entry points
//! (`DllMain`, `DllGetClassObject`, `DllCanUnloadNow`, `DllRegisterServer`,
//! `DllUnregisterServer`). The global module `HMODULE` and the module lock
//! count live here.

#![allow(non_snake_case)]

mod composition;
mod display_attribute;
mod edit_session;
mod factory;
mod guids;
mod key_event_sink;
mod registry;
mod text_input_processor;
mod text_service;
mod thread_mgr_event_sink;

use std::ffi::c_void;
use std::sync::atomic::{AtomicI32, Ordering};

use windows::core::{Interface, BOOL, GUID, HRESULT};
use windows::Win32::Foundation::{CLASS_E_CLASSNOTAVAILABLE, E_POINTER, HMODULE, S_FALSE, S_OK};
use windows::Win32::System::Com::IClassFactory;
use windows::Win32::System::Ole::SELFREG_E_CLASS;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

use crate::factory::ClassFactory;
use crate::guids::GUID_TEXT_SERVICE;

/// The DLL's own module handle, captured in `DllMain`. Used to resolve the
/// DLL's absolute path for `InProcServer32` and the profile icon.
pub(crate) static mut DLL_INSTANCE: HMODULE = HMODULE(std::ptr::null_mut());

/// Module lock count; drives `DllCanUnloadNow`.
static LOCK_COUNT: AtomicI32 = AtomicI32::new(0);

pub(crate) fn lock_module() {
    LOCK_COUNT.fetch_add(1, Ordering::SeqCst);
}

pub(crate) fn unlock_module() {
    LOCK_COUNT.fetch_sub(1, Ordering::SeqCst);
}

/// Returns the captured DLL module handle.
pub(crate) fn dll_instance() -> HMODULE {
    // SAFETY: written exactly once on DLL_PROCESS_ATTACH before any other code
    // in this DLL runs, and only read thereafter.
    unsafe { DLL_INSTANCE }
}

#[no_mangle]
pub extern "system" fn DllMain(hinst: HMODULE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        // SAFETY: single-threaded loader callback; set once.
        unsafe {
            DLL_INSTANCE = hinst;
        }
    }
    // Win32 expects a 4-byte BOOL; TRUE lets the loader proceed.
    BOOL(1)
}

#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if ppv.is_null() {
        return E_POINTER;
    }
    unsafe {
        *ppv = std::ptr::null_mut();
        if *rclsid != GUID_TEXT_SERVICE {
            return CLASS_E_CLASSNOTAVAILABLE;
        }
        // Build the factory and QI it into the out-param. `query` performs the
        // AddRef; the local `factory` handle's Drop performs the matching
        // Release; net refcount on the returned pointer is exactly 1.
        let factory: IClassFactory = ClassFactory.into();
        factory.query(riid, ppv)
    }
}

#[no_mangle]
pub unsafe extern "system" fn DllCanUnloadNow() -> HRESULT {
    if LOCK_COUNT.load(Ordering::SeqCst) <= 0 {
        S_OK
    } else {
        S_FALSE
    }
}

#[no_mangle]
pub unsafe extern "system" fn DllRegisterServer() -> HRESULT {
    match registry::register_all() {
        Ok(()) => S_OK,
        Err(_) => {
            let _ = registry::unregister_all();
            SELFREG_E_CLASS
        }
    }
}

#[no_mangle]
pub unsafe extern "system" fn DllUnregisterServer() -> HRESULT {
    let _ = registry::unregister_all();
    S_OK
}
