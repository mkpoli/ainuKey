const std = @import("std");
const windows = std.os.windows;

const wintype = @import("types.zig");
const UTF16String = wintype.UTF16String;

const win32 = @import("win32");
const Win32Error = win32.Win32Error;

const HKEY = win32.system.registry.HKEY;
pub const REG_SZ = win32.system.registry.REG_SZ;
pub const KEY_WRITE = win32.system.registry.KEY_WRITE;
pub const HKEY_CLASSES_ROOT = win32.system.registry.HKEY_CLASSES_ROOT;
pub const HKEY_LOCAL_MACHINE = win32.system.registry.HKEY_LOCAL_MACHINE;
pub const REG_OPEN_CREATE_OPTIONS = win32.system.registry.REG_OPEN_CREATE_OPTIONS;
pub const REG_OPTION_NON_VOLATILE = 0;

const RegCreateKeyExW = win32.system.registry.RegCreateKeyExW;
const RegSetValueExW = win32.system.registry.RegSetValueExW;
const RegCloseKey = win32.system.registry.RegCloseKey;
pub extern "shlwapi" fn SHDeleteKeyW(hKey: HKEY, pszSubKey: ?windows.LPCWSTR) callconv(windows.WINAPI) windows.LSTATUS;

const WIN32_ERROR = win32.foundation.WIN32_ERROR;

pub fn createAndSetStringValue(hkey: HKEY, sub_key: [:0]const u16, name: ?[:0]const u16, value: [:0]const u16) !void {
    var write_key: ?HKEY = undefined;
    const create_status = RegCreateKeyExW(hkey, sub_key.ptr, 0, null, .{ .VOLATILE = 0 }, KEY_WRITE, null, &write_key, null);
    if (create_status != WIN32_ERROR.NO_ERROR) {
        const err: WIN32_ERROR = create_status;
        switch (err) {
            .ERROR_ACCESS_DENIED => return error.AccessDenied,
            else => return error.Unexpected,
        }
    }
    defer _ = RegCloseKey(write_key);

    // If the data is of type REG_SZ, REG_EXPAND_SZ, or REG_MULTI_SZ, cbData must include the size of the terminating null character or characters.
    // https://docs.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regsetvalueexw
    const data_size_in_bytes = @as(u32, @intCast((value.len + 1) * @sizeOf(u16)));
    const name_ptr: ?windows.LPCWSTR = if (name != null) name.?.ptr else null;

    const value_bytes: *const u8 = @ptrCast(@alignCast(std.mem.sliceAsBytes(value).ptr));

    const set_status = RegSetValueExW(write_key, name_ptr, 0, REG_SZ,
    //  std.mem.sliceAsBytes(value).ptr
    // @alignCast(std.mem.sliceAsBytes(value).ptr)
    value_bytes, data_size_in_bytes);

    if (set_status != WIN32_ERROR.NO_ERROR) {
        const err = set_status;
        switch (err) {
            .ERROR_ACCESS_DENIED => return error.AccessDenied,
            else => return error.Unexpected,
        }
    }
}

/// Wrapper over SHDeleteKeyW that doesn't error if the key is not found
pub fn deleteTree(hkey: HKEY, sub_key: [:0]const u16) !void {
    const status = SHDeleteKeyW(hkey, sub_key);
    if (status != @intFromEnum(WIN32_ERROR.NO_ERROR)) {
        const err = @as(WIN32_ERROR, @enumFromInt(status));
        switch (err) {
            .ERROR_FILE_NOT_FOUND => {}, // no problem
            .ERROR_ACCESS_DENIED => return error.AccessDenied,
            else => return error.Unexpected,
        }
    }
}
