const std = @import("std");

const mem = std.mem;

const win = std.os.windows;
// pub const DLL_INSTANCE_HANDLE: HINSTANCE = null;
const LPCSTR = win.LPCSTR;
const LPCWSTR = win.LPCWSTR;
const HWND = win.HWND;
const UINT = win.UINT;
const WINAPI = win.WINAPI;
const allocator = std.heap.page_allocator;

// /// Used to convert a slice to a null terminated slice on the stack.
// /// TODO https://github.com/ziglang/zig/issues/287
// pub fn toPosixPath(file_path: []const u8) ![MAX_PATH_BYTES - 1:0]u8 {
//     if (std.debug.runtime_safety) assert(std.mem.indexOfScalar(u8, file_path, 0) == null);
//     var path_with_null: [MAX_PATH_BYTES - 1:0]u8 = undefined;
//     // >= rather than > to make room for the null byte
//     if (file_path.len >= MAX_PATH_BYTES) return error.NameTooLong;
//     @memcpy(path_with_null[0..file_path.len], file_path);
//     path_with_null[file_path.len] = 0;
//     return path_with_null;
// }

pub fn toNullTerminatedCString(text: []const u8) !LPCSTR {
    // var allocated: [:0]u8 = try allocator.alloc(u8, text.len + 1);
    // @memcpy(allocated[0..text.len], text);
    // allocated[text.len] = 0;
    // return allocated;

    return try allocator.dupeZ(u8, text);

    // var allocated: []u8 = try allocator.alloc(u8, text.len + 1);
    // @memcpy(allocated, text);
    // allocated[text.len] = 0;
    // return allocated[0..text.len :0];
    // // mem.copyForwards(u8, allocated, text);
    // // allocated[text.len] = 0;
    // // return allocated[0..text.len :0];
}

extern "user32" fn MessageBoxA(hWnd: ?HWND, lpText: LPCSTR, lpCaption: LPCSTR, uType: UINT) callconv(WINAPI) i32;
pub fn messageBox(text: []const u8, caption: LPCSTR) void {
    const text_cstr = toNullTerminatedCString(text) catch {
        _ = MessageBoxA(null, "Failed to allocate memory for message box text", caption, 0);
        return;
    };
    _ = MessageBoxA(null, text_cstr, caption, 0);
}

extern "user32" fn MessageBoxW(hWnd: ?HWND, lpText: LPCWSTR, lpCaption: LPCWSTR, uType: UINT) callconv(WINAPI) i32;
pub fn messageBoxW(text: [:0]const u16, caption: [:0]const u16) void {
    _ = MessageBoxW(null, text, caption, 0);
}
