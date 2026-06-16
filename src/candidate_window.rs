//! A lightweight popup window that renders the candidate list near the caret.
//!
//! Display-only: it uses `WS_EX_NOACTIVATE` + `SW_SHOWNOACTIVATE` so it never
//! steals focus from the composition. The composition layer calls
//! [`CandidateWindow::show`]/[`CandidateWindow::hide`]; the key handler drives
//! selection. All candidate *logic* lives in [`crate::candidates`]; this module
//! is rendering only.
//!
//! Positioning uses `GetGUIThreadInfo` to find the focus caret (robust across
//! apps without an edit-session `GetTextExt` dance); precise per-character
//! placement is a follow-up.
#![allow(dead_code)]

use std::cell::RefCell;
use std::sync::Once;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, ClientToScreen, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect,
    GetStockObject, InvalidateRect, SelectObject, SetBkMode, SetTextColor, UpdateWindow,
    DEFAULT_GUI_FONT, DT_LEFT, DT_SINGLELINE, DT_VCENTER, PAINTSTRUCT, TRANSPARENT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, GetGUIThreadInfo, LoadCursorW,
    RegisterClassExW, SetWindowPos, ShowWindow, GUITHREADINFO, HWND_TOPMOST, IDC_ARROW,
    SWP_NOACTIVATE, SW_HIDE, SW_SHOWNOACTIVATE, WM_PAINT, WNDCLASSEXW, WS_BORDER, WS_EX_NOACTIVATE,
    WS_EX_TOPMOST, WS_POPUP,
};

use crate::dll_instance;

const CLASS_NAME: PCWSTR = w!("AinuKeyCandidateWindow");
const ROW_HEIGHT: i32 = 24;
const WIDTH: i32 = 220;
const PAD: i32 = 4;

thread_local! {
    /// Display strings + selected index, read by the window procedure on paint.
    static PAINT: RefCell<(Vec<String>, usize)> = const { RefCell::new((Vec::new(), 0)) };
}

static REGISTER: Once = Once::new();

fn hinstance() -> HINSTANCE {
    HINSTANCE(dll_instance().0)
}

fn register_class() {
    REGISTER.call_once(|| {
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: hinstance(),
            lpszClassName: CLASS_NAME,
            // SAFETY: IDC_ARROW is a built-in cursor.
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap_or_default() },
            ..Default::default()
        };
        // SAFETY: wc is fully initialized.
        unsafe {
            RegisterClassExW(&wc);
        }
    });
}

/// The candidate popup window.
pub struct CandidateWindow {
    hwnd: HWND,
}

impl CandidateWindow {
    /// Create the (hidden) popup window. `None` if creation fails.
    pub fn new() -> Option<Self> {
        register_class();
        // SAFETY: the class is registered; a popup has no parent.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                CLASS_NAME,
                PCWSTR::null(),
                WS_POPUP | WS_BORDER,
                0,
                0,
                WIDTH,
                ROW_HEIGHT,
                None,
                None,
                Some(hinstance()),
                None,
            )
            .ok()?
        };
        Some(Self { hwnd })
    }

    /// Show `items` with `selected` highlighted, positioned under the caret.
    /// An empty list hides the window.
    pub fn show(&self, items: &[String], selected: usize) {
        if items.is_empty() {
            self.hide();
            return;
        }
        PAINT.with(|p| *p.borrow_mut() = (items.to_vec(), selected));
        let (x, y) = caret_screen_pos();
        let height = ROW_HEIGHT * items.len() as i32 + PAD * 2;
        // SAFETY: hwnd is valid for this object's lifetime.
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                x,
                y,
                WIDTH,
                height,
                SWP_NOACTIVATE,
            );
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            let _ = InvalidateRect(Some(self.hwnd), None, true);
            let _ = UpdateWindow(self.hwnd);
        }
    }

    pub fn hide(&self) {
        // SAFETY: hwnd valid.
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }
}

impl Drop for CandidateWindow {
    fn drop(&mut self) {
        // SAFETY: hwnd was created in `new`.
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

/// Best-effort caret position in screen coordinates (falls back to a corner).
fn caret_screen_pos() -> (i32, i32) {
    let mut gti = GUITHREADINFO {
        cbSize: std::mem::size_of::<GUITHREADINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: gti is sized; thread id 0 = the foreground thread.
    unsafe {
        if GetGUIThreadInfo(0, &mut gti).is_ok() && !gti.hwndCaret.is_invalid() {
            let mut pt = POINT {
                x: gti.rcCaret.left,
                y: gti.rcCaret.bottom,
            };
            if ClientToScreen(gti.hwndCaret, &mut pt).as_bool() {
                return (pt.x, pt.y + 2);
            }
        }
    }
    (120, 120)
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if msg != WM_PAINT {
        return DefWindowProcW(hwnd, msg, wparam, lparam);
    }
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    let mut rc = RECT::default();
    let _ = GetClientRect(hwnd, &mut rc);

    let bg = CreateSolidBrush(COLORREF(0x00FF_FFFF)); // white background
    FillRect(hdc, &rc, bg);
    let _ = DeleteObject(bg.into());

    let font = GetStockObject(DEFAULT_GUI_FONT);
    let old_font = SelectObject(hdc, font);
    SetBkMode(hdc, TRANSPARENT);
    let highlight = CreateSolidBrush(COLORREF(0x00F0_C8A0)); // soft highlight

    PAINT.with(|p| {
        let (items, selected) = &*p.borrow();
        for (i, text) in items.iter().enumerate() {
            let top = PAD + i as i32 * ROW_HEIGHT;
            let row = RECT {
                left: PAD,
                top,
                right: rc.right - PAD,
                bottom: top + ROW_HEIGHT,
            };
            if i == *selected {
                FillRect(hdc, &row, highlight);
            }
            SetTextColor(hdc, COLORREF(0x0020_2020));
            let mut line: Vec<u16> = format!("{}. {}", i + 1, text).encode_utf16().collect();
            let mut text_rc = RECT {
                left: row.left + 6,
                ..row
            };
            DrawTextW(
                hdc,
                &mut line,
                &mut text_rc,
                DT_LEFT | DT_SINGLELINE | DT_VCENTER,
            );
        }
    });

    let _ = DeleteObject(highlight.into());
    SelectObject(hdc, old_font);
    let _ = EndPaint(hwnd, &ps);
    LRESULT(0)
}
