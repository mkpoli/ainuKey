const std = @import("std");
const windows = std.os.windows;
const unicode = std.unicode;

const MAX_PATH = windows.MAX_PATH;
const PATH_MAX_WIDE = windows.PATH_MAX_WIDE;
const HMODULE = windows.HMODULE;

pub const PathBufferW = [PATH_MAX_WIDE]u16;
// pub const PathBufferA = [MAX_PATH]u8;

pub const UTF16String = [:0]u16;
pub const UTF16StringLiteral = [:0]const u16;

pub fn getModuleFileName(dll_instance_handle: HMODULE) !UTF16String {
    var module_file_name_buffer: PathBufferW = undefined;
    return try windows.GetModuleFileNameW(@ptrCast(dll_instance_handle), &module_file_name_buffer, module_file_name_buffer.len);
}

pub fn convertPathWToUTF8(path: UTF16String) ![]u8 {
    var utf8_buffer: [PATH_MAX_WIDE:0]u8 = undefined;
    const len = try unicode.utf16leToUtf8(&utf8_buffer, path);
    return utf8_buffer[0..len];
}

pub fn convertPathWToCStringU8(path: UTF16String) ![PATH_MAX_WIDE:0]u8 {
    var utf8_buffer: [PATH_MAX_WIDE:0]u8 = undefined;
    _ = try unicode.utf16leToUtf8(&utf8_buffer, path);
    return utf8_buffer;

    // return try std.mem.dupe(u8, &utf8_buffer[0..len]);
}
