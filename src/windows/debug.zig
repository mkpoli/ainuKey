const std = @import("std");

const unicode = std.unicode;

const mem = std.mem;

const win = std.os.windows;
// pub const DLL_INSTANCE_HANDLE: HINSTANCE = null;
const LPCSTR = win.LPCSTR;
const LPCWSTR = win.LPCWSTR;
const HWND = win.HWND;
const UINT = win.UINT;
const WINAPI = win.WINAPI;
const allocator = std.heap.page_allocator;

const win32 = @import("win32");
const MessageBoxW = win32.ui.windows_and_messaging.MessageBoxW;
const MB_OK = win32.ui.windows_and_messaging.MB_OK;
const MB_ICONERROR = win32.ui.windows_and_messaging.MB_ICONERROR;
const MB_ICONWARNING = win32.ui.windows_and_messaging.MB_ICONWARNING;
const MB_ICONINFORMATION = win32.ui.windows_and_messaging.MB_ICONINFORMATION;

const MessageBoxType = enum {
    Error,
    Warning,
    Info,
};

pub fn messageBox(text: []const u8, caption: []const u8, box_type: MessageBoxType) void {
    const text_utf16 = unicode.utf8ToUtf16LeAllocZ(allocator, text) catch {
        return;
    };
    const caption_utf16 = unicode.utf8ToUtf16LeAllocZ(allocator, caption) catch {
        return;
    };
    _ = MessageBoxW(null, text_utf16, caption_utf16, switch (box_type) {
        .Error => MB_ICONERROR,
        .Warning => MB_ICONWARNING,
        .Info => MB_ICONINFORMATION,
    });
}
