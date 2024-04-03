const VERSION = @import("version").VERSION;

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

const NAME = consts.NAME;
const LANG = consts.LANG;
const DESC = consts.DESC;
const GUID_TEXT_SERVICE = consts.GUID_TEXT_SERVICE;
const GUID_PROFILE = consts.GUID_PROFILE;
const LOCALE_ID = consts.LOCALE_ID;

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

const ClassFactory = @import("factory.zig").ClassFactory;
const win32 = @import("win32");
const S_OK = win32.foundation.S_OK;
const E_OUTOFMEMORY = win32.foundation.E_OUTOFMEMORY;
const E_NOINTERFACE = win32.foundation.E_NOINTERFACE;
const CLASS_E_CLASSNOTAVAILABLE = win32.foundation.CLASS_E_CLASSNOTAVAILABLE;
const Guid = win32.zig.Guid;
const TextService = @import("service.zig").TextService;
const IID_IClassFactory = win32.system.com.IID_IClassFactory;

const messageBoxWZ = @import("windows/debug.zig").messageBoxWZ;
const mBAP = @import("windows/debug.zig").messageBoxAllocPrint;
const toBraced = @import("windows/guid.zig").toBraced;

export fn DllGetClassObject(
    rclsid: *const Guid,
    riid: *const Guid,
    ppv: ?*?*anyopaque,
) STDAPI {
    messageBox("DllGetClassObject", "ainuKey " ++ VERSION, .Info);
    // messageBox(&toBraced(rclsid.*), "rclsid", .Info);
    // messageBox(&toBraced(riid.*), "riid", .Info);

    // const GUID_TEXT_SERVICE_PTR: *const Guid = &GUID_TEXT_SERVICE;

    if (!std.meta.eql(rclsid.Bytes, GUID_TEXT_SERVICE.Bytes)) {
        mBAP("Unknown CLSID: {s}", .{toBraced(rclsid.*)}, "DllGetClassObject", .Error);
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    if (!std.meta.eql(riid.Bytes, IID_IClassFactory.Bytes)) {
        mBAP("Unknown IID: {s}", .{toBraced(riid.*)}, "DllGetClassObject", .Error);
        return E_NOINTERFACE;
    }

    // messageBox("got object of GUID_TEXT_SERVICE", "DllGetClassObject", .Info);
    ClassFactory.create(std.heap.c_allocator, TextService.create, riid, ppv) catch |e| switch (e) {
        error.NoInterface => {
            mBAP("E_NOINTERFACE: Interface with GUID {s} is not supported!", .{toBraced(riid.*)}, "ClassFactory.create()", .Error);
            return E_NOINTERFACE;
        },
        error.NullPointer => {
            messageBox("E_POINTER: ppv is null", "ClassFactory.create()", .Error);
        },
        error.OutOfMemory => {
            messageBox("E_OUTOFMEMORY: Out of memory", "ClassFactory.create()", .Error);
            return E_OUTOFMEMORY;
        },
        else => {
            messageBox("Unknown error", "ClassFactory.create()", .Error);
        },
    };
    messageBox("created object of GUID_TEXT_SERVICE", "DllGetClassObject", .Info);
    return S_OK;
}

export fn DllRegisterServer() STDAPI {
    messageBox("DllRegisterServer", "ainuKey " ++ VERSION, .Info);
    registry.registerServer(NAME, dll_file_name_w, GUID_TEXT_SERVICE) catch |err| switch (err) {
        error.AccessDenied => return E_ACCESSDENIED,
        error.Unexpected => return E_UNEXPECTED,
    };
    registry.registerProfile(
    // LANG,
    dll_file_name_w, DESC, GUID_TEXT_SERVICE, GUID_PROFILE, LOCALE_ID) catch |err| switch (err) {
        // error.AccessDenied => return E_ACCESSDENIED,
        // error.Unexpected => return E_UNEXPECTED,
        else => return E_UNEXPECTED,
    };
    registry.registerCategories(GUID_TEXT_SERVICE) catch unreachable;
    return 0;
}

export fn DllUnregisterServer() STDAPI {
    registry.unregisterProfile(GUID_TEXT_SERVICE, GUID_PROFILE, LOCALE_ID) catch unreachable;
    registry.unregisterCategories(GUID_TEXT_SERVICE) catch unreachable;
    registry.unregisterServer(GUID_TEXT_SERVICE) catch |err| switch (err) {
        error.AccessDenied => return E_ACCESSDENIED,
        error.Unexpected => return E_UNEXPECTED,
    };
    return 0;
}
