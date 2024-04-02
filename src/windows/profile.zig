const std = @import("std");

const windows = std.os.windows;
const WINAPI = windows.WINAPI;
const HRESULT = windows.HRESULT;

const Guid = win32.zig.Guid;

const CoCreateInstance = win32.system.com.CoCreateInstance;

// const CoCreateInstance = com.CoCreateInstance;

const S_OK = windows.S_OK;
const E_NOINTERFACE = windows.E_NOINTERFACE;
const E_OUTOFMEMORY = windows.E_OUTOFMEMORY;

pub fn createInstanceInproc(T: type, clsid: Guid) ?*T {
    var result: ?*T = null;
    _ = CoCreateInstance(&clsid, null, com.CLSCTX_INPROC_SERVER, &T.IID, @ptrCast(&result));
    return result;
}

const win32 = @import("win32");
const com = win32.system.com;
const ts = win32.ui.text_services;
const IUnknown = win32.system.com.IUnknown;

const CLSCTX_INPROC_SERVER = com.CLSCTX_INPROC_SERVER;

const ITfInputProcessorProfileMgr = ts.ITfInputProcessorProfileMgr;
const IID_ITfInputProcessorProfileMgr = ts.IID_ITfInputProcessorProfileMgr;
const CLSID_TF_InputProcessorProfiles = ts.CLSID_TF_InputProcessorProfiles;
pub fn createProfileManager() ?*ITfInputProcessorProfileMgr {
    var result: ?*ITfInputProcessorProfileMgr = null;
    _ = CoCreateInstance(&CLSID_TF_InputProcessorProfiles, null, CLSCTX_INPROC_SERVER, IID_ITfInputProcessorProfileMgr, @ptrCast(&result));
    return result;
}
