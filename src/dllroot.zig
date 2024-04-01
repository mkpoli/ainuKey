const std = @import("std");
const testing = std.testing;

const win = std.os.windows;

const WINAPI = win.WINAPI;
const HINSTANCE = win.HINSTANCE;
const DWORD = win.DWORD;
const LPVOID = win.LPVOID;
const BOOL = win.BOOL;
const HWND = win.HWND;
const LPCSTR = win.LPCSTR;
const UINT = win.UINT;
const STDAPI = win.HRESULT;
const FALSE = win.FALSE;

// fdwReason parameter values
const DLL_PROCESS_ATTACH: DWORD = 1;
const DLL_THREAD_ATTACH: DWORD = 2;
const DLL_THREAD_DETACH: DWORD = 3;
const DLL_PROCESS_DETACH: DWORD = 0;

const HRESULT = win.HRESULT;
const E_ACCESSDENIED = win.E_ACCESSDENIED;
const E_UNEXPECTED = win.E_UNEXPECTED;

const registry = @import("registry.zig");

const messageBox = @import("windows/debug.zig").messageBox;
const consts = @import("consts.zig");

const GUID = consts.GUID;
const NAME = consts.NAME;
const LANG = consts.LANG;

const wintype = @import("windows/types.zig");
const convertPathWToUTF8 = wintype.convertPathWToUTF8;
const PathBufferW = wintype.PathBufferW;
const UTF16String = wintype.UTF16String;
const getModuleFileName = wintype.getModuleFileName;
// var dll_instance_handle

// var dll_file_name_buffer: PathBufferW = undefined;
pub var dll_file_name_w: UTF16String = undefined;

pub fn DllMain(hinstDLL: HINSTANCE, fdwReason: DWORD, lpReserved: LPVOID) BOOL {
    _ = lpReserved;
    switch (fdwReason) {
        DLL_PROCESS_ATTACH => {
            dll_file_name_w = getModuleFileName(@ptrCast(hinstDLL)) catch {
                return FALSE;
            };
            // dll_file_name_buffer = module_file_name;
            // dll_file_name = dll_file_name_buffer.items;
            // dll_instance_handle = hinstDLL;
            // _ = MessageBoxA(null, "Hello World!", "Zig", 0);
            // messageBox("Hello World!", "Zig");

            // // var dll_file_name_u8
            // const dll_file_name = wintype.convertPathWToCStringU8(dll_file_name_w) catch {
            //     return FALSE;
            // };

            // std.debug.print("DLL File Name: {any}\n", .{dll_file_name});
            // messageBox(@ptrCast(dll_file_name), "DLL File Name");
        },
        DLL_THREAD_ATTACH => {},
        DLL_THREAD_DETACH => {},
        DLL_PROCESS_DETACH => {},
        else => {},
    }
    return 1;
}

export fn DllCanUnloadNow() STDAPI {
    // messageBox("DllCanUnloadNow", "Zig");
    return 0;
}

export fn DllGetClassObject() STDAPI {
    // messageBox("DllGetClassObject", "Zig");
    return 0;
}

export fn DllRegisterServer() STDAPI {
    messageBox("DllRegisterServer", "Zig");
    registry.registerServer(NAME, dll_file_name_w, GUID) catch |err| switch (err) {
        error.AccessDenied => return E_ACCESSDENIED,
        error.Unexpected => return E_UNEXPECTED,
    };
    registry.registerProfile(LANG) catch |err| switch (err) {
        // error.AccessDenied => return E_ACCESSDENIED,
        // error.Unexpected => return E_UNEXPECTED,
        else => return E_UNEXPECTED,
    };
    return 0;
}

export fn DllUnregisterServer() STDAPI {
    messageBox("WTFFFF", "Zig");
    registry.unregisterServer(GUID) catch |err| switch (err) {
        error.AccessDenied => return E_ACCESSDENIED,
        error.Unexpected => return E_UNEXPECTED,
    };
    return 0;
}
