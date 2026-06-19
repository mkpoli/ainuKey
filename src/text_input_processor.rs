//! `ITfTextInputProcessor` + `ITfTextInputProcessorEx`: the activation
//! lifecycle. `Activate` forwards to `ActivateEx`; `ActivateEx` advises the
//! sinks and sets up the display attribute; `Deactivate` tears everything down
//! in reverse, idempotently.

use windows::core::{IUnknownImpl, Interface, Ref};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, ITfCategoryMgr, ITfKeyEventSink, ITfKeystrokeMgr, ITfLangBarItem,
    ITfLangBarItemMgr, ITfSource, ITfTextInputProcessorEx_Impl, ITfTextInputProcessor_Impl,
    ITfThreadMgr, ITfThreadMgrEventSink, TF_INVALID_COOKIE,
};

use crate::guids::GUID_DISPLAY_ATTRIBUTE;
use crate::lang_bar::ModeButton;
use crate::text_service::TextService_Impl;

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, ptim: Ref<'_, ITfThreadMgr>, tid: u32) -> windows::core::Result<()> {
        self.ActivateEx(ptim, tid, 0)
    }

    fn Deactivate(&self) -> windows::core::Result<()> {
        // Defensively cancel any live composition (best-effort; needs a context
        // we no longer have here, so just drop our stored composition handle).
        {
            let mut state = self.inner_mut();
            state.composition = None;
        }

        let thread_mgr = self.inner().thread_mgr.clone();
        if let Some(tm) = thread_mgr.as_ref() {
            // Remove the language-bar item.
            if let Some(item) = self.inner_mut().langbar_item.take() {
                if let Ok(lbim) = tm.cast::<ITfLangBarItemMgr>() {
                    // SAFETY: lbim and item are valid COM interfaces.
                    unsafe {
                        let _ = lbim.RemoveItem(&item);
                    }
                }
            }

            // Unadvise the key-event sink by client id.
            if let Ok(keystroke) = tm.cast::<ITfKeystrokeMgr>() {
                let cid = self.inner().client_id;
                // SAFETY: keystroke is a valid ITfKeystrokeMgr from the TM.
                unsafe {
                    let _ = keystroke.UnadviseKeyEventSink(cid);
                }
            }

            // Unadvise the thread-mgr event sink by cookie.
            let cookie = self.inner().thread_mgr_cookie;
            if cookie != TF_INVALID_COOKIE {
                if let Ok(source) = tm.cast::<ITfSource>() {
                    // SAFETY: source is a valid ITfSource from the TM.
                    unsafe {
                        let _ = source.UnadviseSink(cookie);
                    }
                }
                self.inner_mut().thread_mgr_cookie = TF_INVALID_COOKIE;
            }
        }

        let mut state = self.inner_mut();
        state.category_mgr = None;
        state.display_attribute_atom = 0;
        state.client_id = 0;
        state.buffer.clear();
        state.composition = None;
        state.candidate_window = None;
        state.candidates = crate::candidates::CandidateList::default();
        state.last_committed = None;
        state.prev_committed = None;
        // Release the thread manager LAST.
        state.thread_mgr = None;
        Ok(())
    }
}

impl ITfTextInputProcessorEx_Impl for TextService_Impl {
    fn ActivateEx(
        &self,
        ptim: Ref<'_, ITfThreadMgr>,
        tid: u32,
        _dwflags: u32,
    ) -> windows::core::Result<()> {
        // Idempotency guard: reject re-entry.
        if self.inner().thread_mgr.is_some() {
            return Ok(());
        }

        let thread_mgr = match ptim.as_ref() {
            Some(tm) => tm.clone(),
            None => return Ok(()),
        };

        {
            let mut state = self.inner_mut();
            state.thread_mgr = Some(thread_mgr.clone());
            state.client_id = tid;
        }

        // Run the rest of setup, unwinding fully on any failure.
        if let Err(err) = self.setup(&thread_mgr, tid) {
            let _ = self.Deactivate();
            return Err(err);
        }
        Ok(())
    }
}

impl TextService_Impl {
    fn setup(&self, thread_mgr: &ITfThreadMgr, tid: u32) -> windows::core::Result<()> {
        // Ensure config.toml exists (so it can be found and hand-edited), pick up
        // any on-disk changes, and apply the configured default input mode.
        crate::config::ensure_file();
        let cfg = crate::config::reload();
        self.inner().mode.set(match cfg.input.default_mode {
            crate::config::InputMode::Latin => crate::lang_bar::Mode::Latin,
            crate::config::InputMode::Kana => crate::lang_bar::Mode::Kana,
        });

        // Advise the key-event sink (required to receive keystrokes).
        let keystroke: ITfKeystrokeMgr = thread_mgr.cast()?;
        let this_kes: ITfKeyEventSink = self.to_interface();
        // SAFETY: keystroke / this_kes are valid; fforeground = true.
        unsafe {
            keystroke.AdviseKeyEventSink(tid, &this_kes, true)?;
        }

        // Advise the thread-manager event sink (cheap; observes focus changes).
        let source: ITfSource = thread_mgr.cast()?;
        let this_tmes: ITfThreadMgrEventSink = self.to_interface();
        // SAFETY: source / this_tmes valid; returns a cookie.
        let cookie = unsafe { source.AdviseSink(&ITfThreadMgrEventSink::IID, &this_tmes)? };
        self.inner_mut().thread_mgr_cookie = cookie;

        // Create the category manager and register the display-attribute GUID.
        // SAFETY: standard in-proc COM creation.
        let catmgr: ITfCategoryMgr =
            unsafe { CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)? };
        // SAFETY: catmgr valid; GUID pointer valid.
        let atom = unsafe { catmgr.RegisterGUID(&GUID_DISPLAY_ATTRIBUTE)? };
        {
            let mut state = self.inner_mut();
            state.display_attribute_atom = atom;
            state.category_mgr = Some(catmgr);
            // Initialize composition state to empty.
            state.buffer.clear();
            state.composition = None;
        }

        // Add the mode-switch language-bar button (it shares `mode` with the key
        // handler). Best-effort: a missing language bar must not fail activation.
        let mode = self.inner().mode.clone();
        let item: ITfLangBarItem = ModeButton::new(mode).into();
        if let Ok(lbim) = thread_mgr.cast::<ITfLangBarItemMgr>() {
            // SAFETY: lbim and item are valid COM interfaces.
            unsafe {
                if lbim.AddItem(&item).is_ok() {
                    self.inner_mut().langbar_item = Some(item);
                }
            }
        }

        // Create the candidate window (best-effort; suggestions are optional).
        self.inner_mut().candidate_window = crate::candidate_window::CandidateWindow::new();

        Ok(())
    }
}
