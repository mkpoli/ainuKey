//! `ITfThreadMgrEventSink`: advised so focus changes are observed, but v1 keeps
//! all state in the key handler, so these are trivial stubs.

use windows::core::Ref;
use windows::Win32::UI::TextServices::{
    ITfContext, ITfDocumentMgr, ITfThreadMgrEventSink_Impl,
};

use crate::text_service::TextService_Impl;

impl ITfThreadMgrEventSink_Impl for TextService_Impl {
    fn OnInitDocumentMgr(&self, _pdim: Ref<'_, ITfDocumentMgr>) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnUninitDocumentMgr(&self, _pdim: Ref<'_, ITfDocumentMgr>) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnSetFocus(
        &self,
        _pdimfocus: Ref<'_, ITfDocumentMgr>,
        _pdimprevfocus: Ref<'_, ITfDocumentMgr>,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnPushContext(&self, _pic: Ref<'_, ITfContext>) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnPopContext(&self, _pic: Ref<'_, ITfContext>) -> windows::core::Result<()> {
        Ok(())
    }
}
