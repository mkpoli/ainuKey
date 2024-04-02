const std = @import("std");
const WINAPI = std.os.windows.WINAPI;

const win32 = @import("win32");
// const Guid = win32.

pub const CLSID_TF_InputProcessorProfiles: Guid = Guid.initString("33c53a50-f456-4884-b049-85fd643ecfed");
pub const IID_ITfInputProcessorProfiles: Guid = Guid.initString("1F02B6C5-7842-4EE6-8A0B-9A24183A95CA");

pub const CLSID_TF_CategoryMgr: Guid = Guid.initString("a4b544a1-438d-4b41-9325-869523e2d6c7");
pub const IID_ITfCategoryMgr: Guid = Guid.initString("c3acefb5-f69d-4905-938f-fcadcf4be830");

const HRESULT = i32;

const Guid = win32.zig.Guid;
const IUnknown = win32.system.com.IUnknown;

pub const CLSCTX_INPROC_SERVER: i32 = 1;
pub extern "ole32" fn CoCreateInstance(rclsid: ?*const Guid, punkOuter: ?*IUnknown, dwClsContext: u32, riid: ?*const Guid, ppv: ?*?*anyopaque) callconv(WINAPI) HRESULT;

pub fn createInstanceInproc(T: type, clsid: Guid) ?*T {
    var result: ?*T = null;
    _ = CoCreateInstance(&clsid, null, CLSCTX_INPROC_SERVER, &T.IID, @ptrCast(&result));
    return result;
}
