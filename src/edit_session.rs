//! Synchronous edit-session wrapper. `run_sync` boxes a callback into a
//! non-generic `ITfEditSession`, requests a synchronous read-write session, and
//! returns the callback's result.
//!
//! The session object itself is non-generic (the `#[implement]` macro does not
//! support generic structs cleanly); the typed result is threaded out through a
//! shared `Rc<RefCell<..>>` captured by the boxed closure.

use std::cell::RefCell;
use std::rc::Rc;

use windows::core::implement;
use windows::Win32::Foundation::E_FAIL;
use windows::Win32::UI::TextServices::{
    ITfContext, ITfEditSession, ITfEditSession_Impl, TF_ES_READWRITE, TF_ES_SYNC,
};

/// A type-erased edit-session callback. It captures whatever typed result cell
/// it needs and writes into it when invoked.
type ErasedCallback = Box<dyn FnOnce(u32) -> windows::core::Result<()>>;

#[implement(ITfEditSession)]
pub struct EditSession {
    callback: RefCell<Option<ErasedCallback>>,
}

impl ITfEditSession_Impl for EditSession_Impl {
    fn DoEditSession(&self, ec: u32) -> windows::core::Result<()> {
        if let Some(cb) = self.callback.borrow_mut().take() {
            cb(ec)
        } else {
            Ok(())
        }
    }
}

/// Runs `cb` inside a synchronous, read-write TSF edit session on `context`.
pub fn run_sync<T: 'static>(
    client_id: u32,
    context: &ITfContext,
    cb: impl FnOnce(u32) -> windows::core::Result<T> + 'static,
) -> windows::core::Result<T> {
    let slot: Rc<RefCell<Option<windows::core::Result<T>>>> = Rc::new(RefCell::new(None));
    let slot_inner = Rc::clone(&slot);

    let erased: ErasedCallback = Box::new(move |ec| {
        *slot_inner.borrow_mut() = Some(cb(ec));
        Ok(())
    });

    let obj = EditSession {
        callback: RefCell::new(Some(erased)),
    };
    let session: ITfEditSession = obj.into();

    // SAFETY: context is a valid ITfContext; session is a valid edit session.
    // RequestEditSession returns the inner session HRESULT, which `?` surfaces.
    unsafe {
        context
            .RequestEditSession(client_id, &session, TF_ES_SYNC | TF_ES_READWRITE)?
            .ok()?;
    }

    let result = slot.borrow_mut().take();
    result.unwrap_or_else(|| Err(E_FAIL.into()))
}
