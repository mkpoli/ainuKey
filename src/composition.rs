//! The composition driver: start / update / commit / cancel, the
//! `handle_action` state machine, and `ITfCompositionSink`. This is the engine
//! seam where `ainconv::convert_latn_to_kana` is called.

use std::mem::ManuallyDrop;

use windows::core::{IUnknownImpl, Interface, Ref};
use windows::Win32::Foundation::E_FAIL;
use windows::Win32::System::Variant::{
    VARENUM, VARIANT, VARIANT_0, VARIANT_0_0, VARIANT_0_0_0, VT_I4,
};
use windows::Win32::UI::TextServices::{
    ITfComposition, ITfCompositionSink, ITfCompositionSink_Impl, ITfContext, ITfContextComposition,
    ITfInsertAtSelection, GUID_PROP_ATTRIBUTE, TF_AE_NONE, TF_ANCHOR_END, TF_IAS_QUERYONLY,
    TF_SELECTION, TF_SELECTIONSTYLE, TF_ST_CORRECTION,
};

use crate::edit_session::run_sync;
use crate::key_event_sink::Action;
use crate::text_service::TextService_Impl;

/// Build a `VT_I4` VARIANT holding `value` (the display-attribute atom).
fn variant_i4(value: i32) -> VARIANT {
    VARIANT {
        Anonymous: VARIANT_0 {
            Anonymous: ManuallyDrop::new(VARIANT_0_0 {
                vt: VT_I4,
                wReserved1: 0,
                wReserved2: 0,
                wReserved3: 0,
                Anonymous: VARIANT_0_0_0 { lVal: value },
            }),
        },
    }
}

impl TextService_Impl {
    /// Mutate the buffer and drive the composition for a decoded action.
    pub(crate) fn handle_action(
        &self,
        context: &ITfContext,
        action: Action,
    ) -> windows::core::Result<()> {
        match action {
            Action::Insert(c) => {
                if self.inner().buffer.is_empty() {
                    self.start_composition(context)?;
                }
                self.inner_mut().buffer.push(c);
                self.update_preedit(context)?;
            }
            Action::Backspace => {
                self.inner_mut().buffer.pop();
                if self.inner().buffer.is_empty() {
                    self.cancel(context)?;
                } else {
                    self.update_preedit(context)?;
                }
            }
            Action::Commit => {
                self.commit(context)?;
            }
            Action::Cancel => {
                self.cancel(context)?;
            }
            Action::Passthrough => {}
        }
        Ok(())
    }

    fn start_composition(&self, context: &ITfContext) -> windows::core::Result<()> {
        let cc: ITfContextComposition = context.cast()?;
        let insert: ITfInsertAtSelection = context.cast()?;
        let sink: ITfCompositionSink = self.to_interface();
        let cid = self.inner().client_id;

        let comp = run_sync(cid, context, move |ec| {
            // SAFETY: ec is a valid edit cookie inside the session.
            unsafe {
                let range = insert.InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])?;
                let comp = cc.StartComposition(ec, &range, &sink)?;
                Ok(comp)
            }
        })?;
        self.inner_mut().composition = Some(comp);
        Ok(())
    }

    fn update_preedit(&self, context: &ITfContext) -> windows::core::Result<()> {
        let comp = self.inner().composition.clone().ok_or(E_FAIL)?;
        let kana: Vec<u16> =
            ainconv::convert_latn_to_kana(&crate::romaji::normalize(&self.inner().buffer))
                .encode_utf16()
                .collect();
        let atom = self.inner().display_attribute_atom as i32;
        let cid = self.inner().client_id;
        let ctx = context.clone();

        run_sync(cid, context, move |ec| {
            // SAFETY: ec valid; comp/ctx/range are valid TSF objects.
            unsafe {
                let range = comp.GetRange()?;
                range.SetText(ec, TF_ST_CORRECTION, &kana)?;

                // Apply the underline display attribute over the whole range.
                let prop = ctx.GetProperty(&GUID_PROP_ATTRIBUTE)?;
                let var = variant_i4(atom);
                prop.SetValue(ec, &range, &var)?;

                // Collapse caret to end of preedit.
                range.Collapse(ec, TF_ANCHOR_END)?;
                let selection = TF_SELECTION {
                    range: ManuallyDrop::new(Some(range.clone())),
                    style: TF_SELECTIONSTYLE {
                        ase: TF_AE_NONE,
                        fInterimChar: false.into(),
                    },
                };
                ctx.SetSelection(ec, &[selection])?;
                Ok(())
            }
        })
    }

    fn commit(&self, context: &ITfContext) -> windows::core::Result<()> {
        let comp = match self.inner().composition.clone() {
            Some(c) => c,
            None => return Ok(()),
        };
        let kana: Vec<u16> =
            ainconv::convert_latn_to_kana(&crate::romaji::normalize(&self.inner().buffer))
                .encode_utf16()
                .collect();
        let cid = self.inner().client_id;
        let ctx = context.clone();

        run_sync(cid, context, move |ec| {
            // SAFETY: ec valid; comp/ctx/range valid.
            unsafe {
                let range = comp.GetRange()?;
                range.SetText(ec, TF_ST_CORRECTION, &kana)?;
                ctx.GetProperty(&GUID_PROP_ATTRIBUTE)?.Clear(ec, &range)?;
                range.Collapse(ec, TF_ANCHOR_END)?;
                let selection = TF_SELECTION {
                    range: ManuallyDrop::new(Some(range.clone())),
                    style: TF_SELECTIONSTYLE {
                        ase: TF_AE_NONE,
                        fInterimChar: false.into(),
                    },
                };
                ctx.SetSelection(ec, &[selection])?;
                comp.EndComposition(ec)?;
                Ok(())
            }
        })?;
        let mut state = self.inner_mut();
        state.composition = None;
        state.buffer.clear();
        Ok(())
    }

    fn cancel(&self, context: &ITfContext) -> windows::core::Result<()> {
        let comp = match self.inner().composition.clone() {
            Some(c) => c,
            None => return Ok(()),
        };
        let cid = self.inner().client_id;
        let ctx = context.clone();

        run_sync(cid, context, move |ec| {
            // SAFETY: ec valid; comp/ctx/range valid.
            unsafe {
                let range = comp.GetRange()?;
                range.SetText(ec, TF_ST_CORRECTION, &[])?;
                ctx.GetProperty(&GUID_PROP_ATTRIBUTE)?.Clear(ec, &range)?;
                comp.EndComposition(ec)?;
                Ok(())
            }
        })?;
        let mut state = self.inner_mut();
        state.composition = None;
        state.buffer.clear();
        Ok(())
    }
}

impl ITfCompositionSink_Impl for TextService_Impl {
    fn OnCompositionTerminated(
        &self,
        _ecwrite: u32,
        _pcomposition: Ref<'_, ITfComposition>,
    ) -> windows::core::Result<()> {
        let mut state = self.inner_mut();
        state.composition = None;
        state.buffer.clear();
        Ok(())
    }
}

// Keep VARENUM referenced for clarity of the VT_I4 construction.
const _: VARENUM = VT_I4;
