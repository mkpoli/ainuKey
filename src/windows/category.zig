const std = @import("std");

const windows = std.os.windows;
const WINAPI = windows.WINAPI;
const HRESULT = windows.HRESULT;

const Guid = win32.zig.Guid;
const CoCreateInstance = com.CoCreateInstance;

const win32 = @import("win32");
const com = win32.system.com;
const ts = win32.ui.text_services;
const IUnknown = win32.system.com.IUnknown;

const CLSCTX_INPROC_SERVER = com.CLSCTX_INPROC_SERVER;

pub const ITfCategoryMgr = ts.ITfCategoryMgr;
const IID_ITfCategoryMgr = ts.IID_ITfCategoryMgr;
const CLSID_TF_CategoryMgr = ts.CLSID_TF_CategoryMgr;
pub fn createCategoryManager() ?*ITfCategoryMgr {
    var result: ?*ITfCategoryMgr = null;
    _ = CoCreateInstance(&CLSID_TF_CategoryMgr, null, CLSCTX_INPROC_SERVER, IID_ITfCategoryMgr, @ptrCast(&result));
    return result;
}

pub const GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER: Guid = Guid.initString("046b8c80-1647-40f7-9b21-b93b81aabc1b");
pub const GUID_TFCAT_TIPCAP_COMLESS: Guid = Guid.initString("364215d9-75bc-11d7-a6ef-00065b84435c");
pub const GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT: Guid = Guid.initString("ccf05dd7-4a87-11d7-a6e2-00065b84435c");
pub const GUID_TFCAT_TIPCAP_UIELEMENTENABLED: Guid = Guid.initString("49d2f9cf-1f5e-11d7-a6d3-00065b84435c");
pub const GUID_TFCAT_TIP_KEYBOARD: Guid = Guid.initString("34745c63-b2f0-4784-8b67-5e12c8701a31");
pub const GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT: Guid = Guid.initString("13a016df-560b-46cd-947a-4c3af1e0e35d");
pub const GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT: Guid = Guid.initString("25504fb4-7bab-4bc1-9c69-cf81890f0ef5");
