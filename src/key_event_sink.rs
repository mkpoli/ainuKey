//! `ITfKeyEventSink`: keystroke decode + the "eaten" gate. `OnTestKeyDown`
//! predicts (side-effect-free); `OnKeyDown` performs the edit. v1 registers no
//! preserved keys, so `OnPreservedKey`/`OnTestKeyUp`/`OnKeyUp` report "not
//! eaten".

use windows::core::{Ref, BOOL, GUID};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, GetKeyboardState, ToUnicode, VK_BACK, VK_CONTROL, VK_DOWN, VK_ESCAPE, VK_MENU,
    VK_RETURN, VK_SPACE, VK_UP,
};
use windows::Win32::UI::TextServices::{ITfContext, ITfKeyEventSink_Impl};

use crate::lang_bar::Mode;
use crate::text_service::TextService_Impl;

/// A decoded keystroke action.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// A typed Ainu/Latin character to append to the buffer.
    Insert(char),
    /// Space or Enter — commit the buffer.
    Commit,
    /// Backspace — remove the last char.
    Backspace,
    /// Escape — cancel the composition.
    Cancel,
    /// Down arrow — highlight the next candidate.
    SelectNext,
    /// Up arrow — highlight the previous candidate.
    SelectPrev,
    /// Number key 1-9 — select that candidate (0-based) and commit.
    SelectIndex(usize),
    /// Not for us — pass through to the app.
    Passthrough,
}

/// Returns true if `c` is an "Ainu letter" for input purposes.
fn is_ainu_letter(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '\'' || c == '\u{2019}' || c == '='
}

/// Classify a key down event into an [`Action`].
fn decode(wparam: WPARAM, lparam: LPARAM) -> Action {
    let vk = (wparam.0 & 0xFFFF) as u16;

    // Modifier shortcuts pass through (so Ctrl-C etc. work).
    // SAFETY: GetKeyState is always safe to call.
    let ctrl = unsafe { GetKeyState(VK_CONTROL.0 as i32) } as u16 & 0x8000 != 0;
    let alt = unsafe { GetKeyState(VK_MENU.0 as i32) } as u16 & 0x8000 != 0;
    if ctrl || alt {
        return Action::Passthrough;
    }

    if vk == VK_SPACE.0 || vk == VK_RETURN.0 {
        return Action::Commit;
    }
    if vk == VK_BACK.0 {
        return Action::Backspace;
    }
    if vk == VK_ESCAPE.0 {
        return Action::Cancel;
    }
    if vk == VK_DOWN.0 {
        return Action::SelectNext;
    }
    if vk == VK_UP.0 {
        return Action::SelectPrev;
    }
    // Number keys 1-9 pick a candidate (only eaten while composing).
    if (0x31..=0x39).contains(&vk) {
        return Action::SelectIndex((vk - 0x31) as usize);
    }

    // Resolve a printable char.
    let scan = ((lparam.0 >> 16) & 0xFF) as u32;
    let mut state = [0u8; 256];
    // SAFETY: state is a valid 256-byte buffer.
    if unsafe { GetKeyboardState(&mut state) }.is_err() {
        return Action::Passthrough;
    }
    let mut buf = [0u16; 8];
    // SAFETY: vk/scan are simple integers; state and buf are valid buffers.
    let n = unsafe { ToUnicode(vk as u32, scan, Some(&state), &mut buf, 0) };
    if n == 1 {
        if let Some(c) = char::from_u32(buf[0] as u32) {
            if is_ainu_letter(c) {
                return Action::Insert(c);
            }
        }
    }
    Action::Passthrough
}

impl TextService_Impl {
    /// Pure prediction: the decoded action and whether it would be eaten.
    /// Returning the action lets `OnKeyDown` reuse it instead of decoding twice.
    fn would_eat(&self, wparam: WPARAM, lparam: LPARAM) -> (bool, Action) {
        let action = decode(wparam, lparam);
        let has_composition = !self.inner().buffer.is_empty();
        let eaten = match action {
            // In Latin mode we eat nothing, so letters pass straight through to
            // the app; in Kana mode we capture them to build the composition.
            Action::Insert(_) => self.inner().mode.get() == Mode::Kana,
            Action::Commit
            | Action::Backspace
            | Action::Cancel
            | Action::SelectNext
            | Action::SelectPrev
            | Action::SelectIndex(_) => has_composition,
            Action::Passthrough => false,
        };
        (eaten, action)
    }
}

impl ITfKeyEventSink_Impl for TextService_Impl {
    fn OnSetFocus(&self, _fforeground: BOOL) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnTestKeyDown(
        &self,
        _pic: Ref<'_, ITfContext>,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> windows::core::Result<BOOL> {
        Ok(self.would_eat(wparam, lparam).0.into())
    }

    fn OnTestKeyUp(
        &self,
        _pic: Ref<'_, ITfContext>,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> windows::core::Result<BOOL> {
        Ok(false.into())
    }

    fn OnKeyDown(
        &self,
        pic: Ref<'_, ITfContext>,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> windows::core::Result<BOOL> {
        let context = match pic.as_ref() {
            Some(c) => c.clone(),
            None => return Ok(false.into()),
        };
        let (eaten, action) = self.would_eat(wparam, lparam);
        if eaten {
            self.handle_action(&context, action)?;
        }
        Ok(eaten.into())
    }

    fn OnKeyUp(
        &self,
        _pic: Ref<'_, ITfContext>,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> windows::core::Result<BOOL> {
        Ok(false.into())
    }

    fn OnPreservedKey(
        &self,
        _pic: Ref<'_, ITfContext>,
        _rguid: *const GUID,
    ) -> windows::core::Result<BOOL> {
        Ok(false.into())
    }
}
