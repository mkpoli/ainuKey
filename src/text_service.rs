//! The master TSF COM object. A single `TextService` struct implements every
//! interface the running TIP needs (single COM object, single refcount), plus
//! the inner `RefCell` state.

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::rc::Rc;

use windows::core::implement;
use windows::Win32::UI::TextServices::{
    ITfCategoryMgr, ITfComposition, ITfCompositionSink, ITfDisplayAttributeProvider,
    ITfKeyEventSink, ITfLangBarItem, ITfTextInputProcessor, ITfTextInputProcessorEx, ITfThreadMgr,
    ITfThreadMgrEventSink, TF_INVALID_COOKIE,
};

use crate::lang_bar::Mode;

/// Inner, single-threaded-apartment state. All `_Impl` methods take `&self`;
/// mutation goes through `RefCell::borrow_mut`.
pub struct TextServiceState {
    /// The thread manager handed to us in `ActivateEx`.
    pub thread_mgr: Option<ITfThreadMgr>,
    /// TF_CLIENTID assigned at activation.
    pub client_id: u32,
    /// Cookie for the advised thread-manager event sink.
    pub thread_mgr_cookie: u32,
    /// Category manager (created at activation; used to register the display
    /// attribute GUID and obtain its atom).
    pub category_mgr: Option<ITfCategoryMgr>,
    /// TfGuidAtom for our display attribute (from `RegisterGUID`).
    pub display_attribute_atom: u32,
    /// The live composition, if any.
    pub composition: Option<ITfComposition>,
    /// The running romaji buffer.
    pub buffer: String,
    /// Current input mode, shared with the language-bar button.
    pub mode: Rc<Cell<Mode>>,
    /// The language-bar item, kept so it can be removed at deactivation.
    pub langbar_item: Option<ITfLangBarItem>,
}

impl Default for TextServiceState {
    fn default() -> Self {
        Self {
            thread_mgr: None,
            client_id: 0,
            thread_mgr_cookie: TF_INVALID_COOKIE,
            category_mgr: None,
            display_attribute_atom: 0,
            composition: None,
            buffer: String::new(),
            mode: Rc::new(Cell::new(Mode::Kana)),
            langbar_item: None,
        }
    }
}

#[implement(
    ITfTextInputProcessor,
    ITfTextInputProcessorEx,
    ITfKeyEventSink,
    ITfThreadMgrEventSink,
    ITfCompositionSink,
    ITfDisplayAttributeProvider
)]
pub struct TextService {
    inner: RefCell<TextServiceState>,
}

impl TextService {
    pub fn new() -> Self {
        Self {
            inner: RefCell::new(TextServiceState::default()),
        }
    }
}

/// Shared helpers used by the per-interface impls. These are implemented on the
/// macro-generated `TextService_Impl` type, since that is the `&self` seen
/// inside `_Impl` trait methods.
impl TextService_Impl {
    pub(crate) fn inner(&self) -> Ref<'_, TextServiceState> {
        self.inner.borrow()
    }

    pub(crate) fn inner_mut(&self) -> RefMut<'_, TextServiceState> {
        self.inner.borrow_mut()
    }
}
