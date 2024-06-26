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

const ITfCategoryMgr = ts.ITfCategoryMgr;
const IID_ITfCategoryMgr = ts.IID_ITfCategoryMgr;
const CLSID_TF_CategoryMgr = ts.CLSID_TF_CategoryMgr;
fn createCategoryManager() ?*ITfCategoryMgr {
    var result: ?*ITfCategoryMgr = null;
    _ = CoCreateInstance(&CLSID_TF_CategoryMgr, null, CLSCTX_INPROC_SERVER, IID_ITfCategoryMgr, @ptrCast(&result));
    return result;
}

const S_OK = windows.S_OK;

pub fn registerCategories(guid_text_service: Guid, categories: []const Guid) !void {
    const category_manager = createCategoryManager() orelse {
        return error.CategoryManagerCreationFailure;
    };

    for (categories) |category_guid| {
        const result = ITfCategoryMgr.ITfCategoryMgr_RegisterCategory(category_manager, &guid_text_service, &category_guid, &guid_text_service);
        switch (result) {
            S_OK => {},
            else => {
                return error.CategoryRegistrationFailure;
            },
        }
    }
}

pub fn unregisterCategories(guid_text_service: Guid, categories: []const Guid) !void {
    const category_manager = createCategoryManager() orelse {
        return error.CategoryManagerCreationFailure;
    };

    for (categories) |category_guid| {
        const result = ITfCategoryMgr.ITfCategoryMgr_UnregisterCategory(category_manager, &guid_text_service, &category_guid, &guid_text_service);
        switch (result) {
            S_OK => {},
            else => {
                return error.CategoryUnregistrationFailure;
            },
        }
    }
}
