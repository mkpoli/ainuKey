//! A small native Win32 settings dialog, opened from the Start-menu shortcut
//! (`ShowSettings`) and `ITfFnConfigure::Show`.
//!
//! Controls are bound one-to-one to [`crate::config::Config`]: the dialog loads
//! the current config, and on **OK** reads every control back, saves, and
//! [`crate::config::reload`]s. This is the GUI half of the two-way TOML config —
//! the file can still be hand-edited (and is self-documenting), and the dialog
//! now exposes the *same* options the file does, so the two are at parity.
//!
//! Display-only Win32 (no resource compiler): a popup window with `BUTTON`,
//! `COMBOBOX`, `EDIT` and `STATIC` controls grouped under `BS_GROUPBOX` frames,
//! driven by a nested modal message loop (mirroring how `DialogBox` works).

use std::ffi::c_void;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{GetStockObject, DEFAULT_GUI_FONT};
use windows::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetDlgItem, GetMessageW,
    GetSystemMetrics, GetWindowTextW, IsDialogMessageW, IsWindow, LoadCursorW, RegisterClassExW,
    SendMessageW, SetForegroundWindow, SetWindowPos, ShowWindow, TranslateMessage, BM_GETCHECK,
    BM_SETCHECK, BS_AUTOCHECKBOX, BS_DEFPUSHBUTTON, BS_GROUPBOX, BS_PUSHBUTTON, CBS_DROPDOWNLIST,
    CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL, ES_NUMBER, HMENU, IDCANCEL, IDC_ARROW, IDOK, MSG,
    SM_CXSCREEN, SM_CYSCREEN, SWP_NOSIZE, SW_SHOW, WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_CREATE,
    WM_SETFONT, WNDCLASSEXW, WS_BORDER, WS_CAPTION, WS_CHILD, WS_POPUP, WS_SYSMENU, WS_TABSTOP,
    WS_VISIBLE, WS_VSCROLL,
};

use crate::config::{Config, InputMode, TuStyle};

const CLASS_NAME: PCWSTR = w!("AinuKeySettingsDialog");

// Control IDs (OK/Cancel use the standard IDOK/IDCANCEL; static/group frames use 0).
const ID_LATIN: i32 = 1001; // input.default_mode == Latin
const ID_ROMAJI: i32 = 1002; // input.romaji_input_mode
const ID_TU_COMBO: i32 = 1003; // orthography.tu_style (4-way)
const ID_SMALL_I: i32 = 1010; // orthography.use_small_i
const ID_SMALL_U: i32 = 1011; // orthography.use_small_u
const ID_SMALL_N: i32 = 1012; // orthography.use_small_n
const ID_WI: i32 = 1013; // orthography.use_wi
const ID_WE: i32 = 1014; // orthography.use_we
const ID_WO: i32 = 1015; // orthography.use_wo
const ID_EQUALS: i32 = 1016; // orthography.show_equals_boundary
const ID_SUGGEST: i32 = 1020; // suggestions.enabled
const ID_CONTEXT: i32 = 1021; // suggestions.context_aware
const ID_MAXCAND: i32 = 1022; // suggestions.max_candidates (edit)

// The tu-style dropdown order; index maps to TuStyle in both directions.
const TU_ITEMS: [(PCWSTR, TuStyle); 4] = [
    (w!("ト゚  (to)"), TuStyle::To),
    (w!("ツ゚  (tsu)"), TuStyle::Tsu),
    (w!("トゥ  (twu)"), TuStyle::Twu),
    (w!("ツ  (plain)"), TuStyle::PlainTsu),
];

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
    let (w, h) = (470, 584);
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

/// Build the grouped controls, loaded from the current config.
unsafe fn build_controls(hwnd: HWND) {
    let cfg = Config::load();
    let pad = 14; // group inset from the window edge
    let cw = 424; // group width
    let gx = pad + 14; // control x inside a group
    let cbw = cw - 28; // checkbox width inside a group
    let checkbox = WINDOW_STYLE(BS_AUTOCHECKBOX as u32) | WS_TABSTOP;

    // Helpers (closures re-enter `unsafe`; they don't inherit the fn's context).
    let group = |label: PCWSTR, y: i32, h: i32| unsafe {
        child(hwnd, w!("BUTTON"), label, WINDOW_STYLE(BS_GROUPBOX as u32), (pad, y, cw, h), 0);
    };
    let check = |id: i32, label: PCWSTR, y: i32, on: bool| unsafe {
        let c = child(hwnd, w!("BUTTON"), label, checkbox, (gx, y, cbw, 22), id);
        SendMessageW(c, BM_SETCHECK, Some(WPARAM(on as usize)), None);
    };
    let label = |text: PCWSTR, x: i32, y: i32, w: i32| unsafe {
        child(hwnd, w!("STATIC"), text, WINDOW_STYLE(0), (x, y, w, 20), 0);
    };

    let mut y = 8;

    // --- Input ---
    group(w!("入力 / Input"), y, 80);
    check(
        ID_LATIN,
        w!("ローマ字モードで開始 / Start in Latin mode"),
        y + 22,
        cfg.input.default_mode == InputMode::Latin,
    );
    check(
        ID_ROMAJI,
        w!("ローマ字入力モード（確定時に変換）/ Romaji input mode"),
        y + 48,
        cfg.input.romaji_input_mode,
    );
    y += 92;

    // --- Orthography ---
    group(w!("表記 / Orthography"), y, 264);
    let mut oy = y + 24;
    label(w!("「tu」の表記 / tu rendering:"), gx, oy + 4, 160);
    let combo = child(
        hwnd,
        w!("COMBOBOX"),
        w!(""),
        WINDOW_STYLE(CBS_DROPDOWNLIST as u32) | WS_TABSTOP | WS_VSCROLL,
        (gx + 164, oy, 218, 200),
        ID_TU_COMBO,
    );
    let mut sel = 0usize;
    for (i, (text, style)) in TU_ITEMS.iter().enumerate() {
        SendMessageW(combo, CB_ADDSTRING, Some(WPARAM(0)), Some(LPARAM(text.0 as isize)));
        if cfg.orthography.tu_style == *style {
            sel = i;
        }
    }
    SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(sel)), None);
    oy += 34;
    for (id, text, on) in [
        (ID_SMALL_I, w!("-y を小書き ィ で表記 / -y coda as small ィ"), cfg.orthography.use_small_i),
        (ID_SMALL_U, w!("-w を小書き ゥ で表記 / -w coda as small ゥ"), cfg.orthography.use_small_u),
        (ID_SMALL_N, w!("-n を小書き ㇴ で表記 / -n coda as small ㇴ"), cfg.orthography.use_small_n),
        (ID_WI, w!("wi を ヰ で表記 / wi as ヰ"), cfg.orthography.use_wi),
        (ID_WE, w!("we を ヱ で表記 / we as ヱ"), cfg.orthography.use_we),
        (ID_WO, w!("wo を ヲ で表記 / wo as ヲ"), cfg.orthography.use_wo),
        (ID_EQUALS, w!("「=」の境界を表示 / Show the = boundary"), cfg.orthography.show_equals_boundary),
    ] {
        check(id, text, oy, on);
        oy += 26;
    }
    y += 276;

    // --- Suggestions ---
    group(w!("変換候補 / Suggestions"), y, 96);
    check(ID_SUGGEST, w!("変換候補を表示 / Show word suggestions"), y + 22, cfg.suggestions.enabled);
    check(ID_CONTEXT, w!("文脈で並べ替え / Context-aware ranking"), y + 48, cfg.suggestions.context_aware);
    label(w!("最大候補数 / Max candidates:"), gx, y + 76, 190);
    let maxtext: Vec<u16> = format!("{}\0", cfg.suggestions.max_candidates)
        .encode_utf16()
        .collect();
    child(
        hwnd,
        w!("EDIT"),
        PCWSTR(maxtext.as_ptr()),
        WINDOW_STYLE(ES_NUMBER as u32) | WS_TABSTOP | WS_BORDER,
        (gx + 200, y + 74, 56, 22),
        ID_MAXCAND,
    );
    y += 108;

    // --- Buttons ---
    child(
        hwnd,
        w!("BUTTON"),
        w!("OK"),
        WINDOW_STYLE(BS_DEFPUSHBUTTON as u32) | WS_TABSTOP,
        (pad + cw - 210, y, 95, 28),
        IDOK.0,
    );
    child(
        hwnd,
        w!("BUTTON"),
        w!("キャンセル / Cancel"),
        WINDOW_STYLE(BS_PUSHBUTTON as u32) | WS_TABSTOP,
        (pad + cw - 105, y, 105, 28),
        IDCANCEL.0,
    );
}

/// Read every control, save over the on-disk config, and refresh the running one.
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

    // tu_style from the dropdown index (falls back to the current value on error).
    if let Ok(combo) = GetDlgItem(Some(hwnd), ID_TU_COMBO) {
        let i = SendMessageW(combo, CB_GETCURSEL, None, None).0;
        if let Some((_, style)) = TU_ITEMS.get(i as usize) {
            cfg.orthography.tu_style = *style;
        }
    }
    cfg.orthography.use_small_i = checked(ID_SMALL_I);
    cfg.orthography.use_small_u = checked(ID_SMALL_U);
    cfg.orthography.use_small_n = checked(ID_SMALL_N);
    cfg.orthography.use_wi = checked(ID_WI);
    cfg.orthography.use_we = checked(ID_WE);
    cfg.orthography.use_wo = checked(ID_WO);
    cfg.orthography.show_equals_boundary = checked(ID_EQUALS);

    cfg.suggestions.enabled = checked(ID_SUGGEST);
    cfg.suggestions.context_aware = checked(ID_CONTEXT);
    // Max candidates: keep the existing value if the field is blank/zero/garbage.
    if let Some(n) = read_number(hwnd, ID_MAXCAND) {
        if n >= 1 {
            cfg.suggestions.max_candidates = n;
        }
    }

    let _ = cfg.save();
    crate::config::reload();
}

/// Read an EDIT control's text as a `usize`, if it parses.
unsafe fn read_number(hwnd: HWND, id: i32) -> Option<usize> {
    let c = GetDlgItem(Some(hwnd), id).ok()?;
    let mut buf = [0u16; 16];
    let n = GetWindowTextW(c, &mut buf) as usize;
    String::from_utf16_lossy(&buf[..n]).trim().parse().ok()
}
