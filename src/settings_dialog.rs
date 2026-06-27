//! A small native Win32 settings dialog, opened from `ITfFnConfigure::Show`.
//!
//! Checkboxes bound to [`crate::config::Config`]: load the current config into
//! the controls, and on **OK** read them back, merge over the on-disk config
//! (preserving fields not exposed here, e.g. `max_candidates`), save, and
//! [`crate::config::reload`]. This is the GUI half of the two-way TOML config —
//! the file can still be hand-edited and is picked up on the next activation.
//!
//! Display-only Win32 (no resource compiler): a popup window with `BUTTON`
//! controls and a nested modal message loop, mirroring how `DialogBox` works.

use std::ffi::c_void;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetStockObject, DEFAULT_GUI_FONT};
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetDlgItem, GetMessageW,
    GetSystemMetrics, IsDialogMessageW, IsWindow, LoadCursorW, RegisterClassExW, SendMessageW,
    SetForegroundWindow, SetWindowPos, ShowWindow, TranslateMessage, BM_GETCHECK, BM_SETCHECK,
    BS_AUTOCHECKBOX, BS_DEFPUSHBUTTON, BS_PUSHBUTTON, HMENU, IDCANCEL, IDC_ARROW, IDOK, MSG,
    SM_CXSCREEN, SM_CYSCREEN, SWP_NOSIZE, SW_SHOW, WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_CREATE,
    WM_SETFONT, WNDCLASSEXW, WS_CAPTION, WS_CHILD, WS_GROUP, WS_POPUP, WS_SYSMENU, WS_TABSTOP,
    WS_VISIBLE,
};

use crate::config::{Config, InputMode, TuStyle};

const CLASS_NAME: PCWSTR = w!("AinuKeySettingsDialog");

// Checkbox control IDs (OK/Cancel use the standard IDOK/IDCANCEL).
const ID_LATIN: i32 = 1001;
const ID_ROMAJI: i32 = 1002;
const ID_TSU: i32 = 1003;
const ID_GLIDES: i32 = 1004;
const ID_EQUALS: i32 = 1005;
const ID_SUGGEST: i32 = 1006;

/// (id, trilingual label, initial-checked from config).
fn checkbox_specs(cfg: &Config) -> [(i32, PCWSTR, bool); 6] {
    [
        (
            ID_LATIN,
            w!("ローマ字モードで開始 / Start in Latin mode"),
            cfg.input.default_mode == InputMode::Latin,
        ),
        (
            ID_ROMAJI,
            w!("ローマ字入力モード（確定時に変換）/ Romaji input mode"),
            cfg.input.romaji_input_mode,
        ),
        (
            ID_TSU,
            w!("「tu」を ツ゚ で表記（ト゚ の代わり）/ Use ツ゚ for tu"),
            cfg.orthography.tu_style == TuStyle::Tsu,
        ),
        (
            ID_GLIDES,
            w!("y/w の半母音を ィ/ゥ で表記 / Small glides ィ/ゥ"),
            // Checked only when both codas are on (the box represents both-or-none).
            cfg.orthography.use_small_i && cfg.orthography.use_small_u,
        ),
        (
            ID_EQUALS,
            w!("「=」の境界を表示 / Show the = boundary"),
            cfg.orthography.show_equals_boundary,
        ),
        (
            ID_SUGGEST,
            w!("変換候補を表示 / Word suggestions"),
            cfg.suggestions.enabled,
        ),
    ]
}

fn gui_font() -> WPARAM {
    // SAFETY: DEFAULT_GUI_FONT is a built-in stock object.
    WPARAM(unsafe { GetStockObject(DEFAULT_GUI_FONT).0 } as usize)
}

/// Open the modal settings dialog over `parent`.
pub fn show(parent: HWND) {
    register_class();
    let hinst = crate::dll_instance();

    // Title carries the build version so users can confirm which build is running
    // (matches the DLL's VERSIONINFO; both come from CARGO_PKG_VERSION).
    let title: Vec<u16> = format!("ainuKey v{} — 設定 / Settings\0", env!("CARGO_PKG_VERSION"))
        .encode_utf16()
        .collect();

    // Centre a fixed-size dialog on the primary screen.
    let (w, h) = (440, 290);
    // SAFETY: GetSystemMetrics is always safe.
    let (sx, sy) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    let (x, y) = ((sx - w).max(0) / 2, (sy - h).max(0) / 2);

    // SAFETY: class is registered; parent may be null (no owner).
    let hwnd = unsafe {
        CreateWindowExW(
            Default::default(),
            CLASS_NAME,
            PCWSTR(title.as_ptr()),
            WS_POPUP | WS_CAPTION | WS_SYSMENU,
            x,
            y,
            w,
            h,
            Some(parent),
            None,
            Some(hinst.into()),
            None,
        )
    };
    let Ok(hwnd) = hwnd else { return };

    // SAFETY: hwnd valid; standard modal-popup dance.
    unsafe {
        let parent_was_enabled = !parent.is_invalid();
        if parent_was_enabled {
            let _ = EnableWindow(parent, false);
        }
        let _ = SetWindowPos(hwnd, None, x, y, 0, 0, SWP_NOSIZE);
        let _ = ShowWindow(hwnd, SW_SHOW);
        // Pop to the front — without this the dialog can open behind the active
        // window (especially when launched from the Start-menu rundll32 shortcut).
        let _ = SetForegroundWindow(hwnd);

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if !IsDialogMessageW(hwnd, &msg).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if !IsWindow(Some(hwnd)).as_bool() {
                break;
            }
        }

        if parent_was_enabled {
            let _ = EnableWindow(parent, true);
            let _ = SetForegroundWindow(parent);
        }
    }
}

fn register_class() {
    use std::sync::Once;
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: Some(wndproc),
            hInstance: crate::dll_instance().into(),
            lpszClassName: CLASS_NAME,
            // SAFETY: IDC_ARROW is a built-in cursor; (HBRUSH) COLOR_BTNFACE+1 background.
            hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap_or_default() },
            hbrBackground: windows::Win32::Graphics::Gdi::HBRUSH(
                (windows::Win32::Graphics::Gdi::COLOR_BTNFACE.0 + 1) as isize as *mut c_void,
            ),
            ..Default::default()
        };
        // SAFETY: wc is fully initialized.
        unsafe {
            RegisterClassExW(&wc);
        }
    });
}

/// Create a child control of class `class` with `id`, positioned at the given
/// rect, using the GUI font.
unsafe fn child(
    parent: HWND,
    class: PCWSTR,
    text: PCWSTR,
    style: WINDOW_STYLE,
    (x, y, w, h): (i32, i32, i32, i32),
    id: i32,
) -> HWND {
    let hwnd = CreateWindowExW(
        Default::default(),
        class,
        text,
        WS_CHILD | WS_VISIBLE | style,
        x,
        y,
        w,
        h,
        Some(parent),
        Some(HMENU(id as isize as *mut c_void)),
        Some(crate::dll_instance().into()),
        None,
    )
    .unwrap_or_default();
    if !hwnd.is_invalid() {
        SendMessageW(hwnd, WM_SETFONT, Some(gui_font()), Some(LPARAM(1)));
    }
    hwnd
}

extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: standard window-procedure callbacks; hwnd is our window.
    unsafe {
        match msg {
            WM_CREATE => {
                build_controls(hwnd);
                LRESULT(0)
            }
            WM_COMMAND => {
                match (wparam.0 & 0xFFFF) as i32 {
                    id if id == IDOK.0 => {
                        apply(hwnd);
                        let _ = DestroyWindow(hwnd);
                    }
                    id if id == IDCANCEL.0 => {
                        let _ = DestroyWindow(hwnd);
                    }
                    _ => {}
                }
                LRESULT(0)
            }
            WM_CLOSE => {
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

/// Create the checkboxes (loaded from the current config) and the buttons.
unsafe fn build_controls(hwnd: HWND) {
    let cfg = Config::load();
    let (mut y, row, pad, cw) = (12, 34, 16, 408);
    for (id, label, checked) in checkbox_specs(&cfg) {
        let cb = child(
            hwnd,
            w!("BUTTON"),
            label,
            WINDOW_STYLE(BS_AUTOCHECKBOX as u32) | WS_TABSTOP | WS_GROUP,
            (pad, y, cw, 24),
            id,
        );
        SendMessageW(
            cb,
            BM_SETCHECK,
            Some(WPARAM(if checked { 1 } else { 0 })),
            None,
        );
        y += row;
    }
    y += 8;
    child(
        hwnd,
        w!("BUTTON"),
        w!("OK"),
        WINDOW_STYLE(BS_DEFPUSHBUTTON as u32) | WS_TABSTOP,
        (cw + pad - 200, y, 95, 26),
        IDOK.0,
    );
    child(
        hwnd,
        w!("BUTTON"),
        w!("キャンセル / Cancel"),
        WINDOW_STYLE(BS_PUSHBUTTON as u32) | WS_TABSTOP,
        (cw + pad - 100, y, 100, 26),
        IDCANCEL.0,
    );
}

/// Read the checkboxes, merge over the on-disk config (preserving unexposed
/// fields), save, and refresh the running config.
unsafe fn apply(hwnd: HWND) {
    let checked = |id: i32| -> bool {
        match GetDlgItem(Some(hwnd), id) {
            Ok(c) => SendMessageW(c, BM_GETCHECK, None, None).0 == 1,
            Err(_) => false,
        }
    };

    let mut cfg = Config::load();
    cfg.input.default_mode = if checked(ID_LATIN) {
        InputMode::Latin
    } else {
        InputMode::Kana
    };
    cfg.input.romaji_input_mode = checked(ID_ROMAJI);
    cfg.orthography.tu_style = if checked(ID_TSU) {
        TuStyle::Tsu
    } else {
        TuStyle::To
    };
    // The single "small glides" checkbox can only represent both-on or both-off,
    // so it must not clobber an asymmetric per-coda state set via the config file
    // (e.g. use_small_i=true, use_small_u=false). Write both fields only when they
    // already agree, or when the user turns the option on.
    let glides = checked(ID_GLIDES);
    if glides || cfg.orthography.use_small_i == cfg.orthography.use_small_u {
        cfg.orthography.use_small_i = glides;
        cfg.orthography.use_small_u = glides;
    }
    cfg.orthography.show_equals_boundary = checked(ID_EQUALS);
    cfg.suggestions.enabled = checked(ID_SUGGEST);

    let _ = cfg.save();
    crate::config::reload();
}
