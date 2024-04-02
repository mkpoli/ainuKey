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
fn createProfileManager() ?*ITfInputProcessorProfileMgr {
    var result: ?*ITfInputProcessorProfileMgr = null;
    _ = CoCreateInstance(&CLSID_TF_InputProcessorProfiles, null, CLSCTX_INPROC_SERVER, IID_ITfInputProcessorProfileMgr, @ptrCast(&result));
    return result;
}

pub fn registerProfile(guid_text_service: Guid, locale_id: u16, guid_profile: Guid, description: []const u16, icon_path: []const u16, icon_index: u32, hkl_substitude: ?ts.HKL, preferred_layout: u32, enabled_by_default: bool, flags: u32) !void {
    const profile_manager = createProfileManager() orelse {
        return error.ProfileManagerCreationFailure;
    };

    const result = ITfInputProcessorProfileMgr.ITfInputProcessorProfileMgr_RegisterProfile(profile_manager, &guid_text_service, locale_id, &guid_profile, @ptrCast(description.ptr), @intCast(description.len), @ptrCast(icon_path.ptr), @intCast(icon_path.len), icon_index, hkl_substitude, preferred_layout, @intFromBool(enabled_by_default), flags);

    switch (result) {
        S_OK => {},
        else => {
            return error.ProfileRegistrationFailure;
        },
    }
}

pub fn unregisterProfile(guid_text_service: Guid, locale_id: u16, guid_profile: Guid, flags: u32) !void {
    const profile_manager = createProfileManager() orelse {
        return error.ProfileManagerCreationFailure;
    };

    const result = ITfInputProcessorProfileMgr.ITfInputProcessorProfileMgr_UnregisterProfile(profile_manager, &guid_text_service, locale_id, &guid_profile, flags);

    switch (result) {
        S_OK => {},
        else => {
            return error.ProfileUnregistrationFailure;
        },
    }
}
