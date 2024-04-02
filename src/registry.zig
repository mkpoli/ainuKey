const std = @import("std");
const windows = std.os.windows;
const unicode = std.unicode;

const wintype = @import("windows/types.zig");
const UTF16String = wintype.UTF16String;
const UTF16StringLiteral = wintype.UTF16StringLiteral;

const registry = @import("windows/registry.zig");
const HKEY_CLASSES_ROOT = registry.HKEY_CLASSES_ROOT;

const profile = @import("windows/profile.zig");

const to16 = unicode.utf8ToUtf16LeStringLiteral;

const CLSID: UTF16StringLiteral = to16("CLSID\\");
const InprocServer32: UTF16StringLiteral = to16("\\InprocServer32");

const messageBox = @import("windows/debug.zig").messageBox;
const messageBoxWZ = @import("windows/debug.zig").messageBoxWZ;

const toBraced = @import("windows/guid.zig").toBraced;

pub fn registerServer(comptime service_name: UTF16StringLiteral, dll_path: UTF16String, comptime guid: Guid) !void {
    messageBox("Registering server", "registerServer", .Info);
    const clsid_key: UTF16StringLiteral = CLSID ++ toBraced(guid);
    const inproc_key = CLSID ++ toBraced(guid) ++ InprocServer32;

    messageBoxWZ(clsid_key, &[_:0]u16{}, .Info);
    messageBoxWZ(inproc_key, &[_:0]u16{}, .Info);
    try registry.createAndSetStringValue(HKEY_CLASSES_ROOT, clsid_key, null, service_name);
    try registry.createAndSetStringValue(HKEY_CLASSES_ROOT, inproc_key, null, dll_path);
    const threading_model_name = std.unicode.utf8ToUtf16LeStringLiteral("ThreadingModel");
    const threading_model_value = std.unicode.utf8ToUtf16LeStringLiteral("Apartment");
    try registry.createAndSetStringValue(HKEY_CLASSES_ROOT, inproc_key, threading_model_name, threading_model_value);

    messageBox("Server registered", "registerServer", .Info);
}

pub fn unregisterServer(comptime guid: Guid) !void {
    const clsid_key = CLSID ++ toBraced(guid);
    try registry.deleteTree(HKEY_CLASSES_ROOT, clsid_key);
}

const win32 = @import("win32");

const GetLocaleInfoEx = win32.globalization.GetLocaleInfoEx;
const LocaleNameToLCID = win32.globalization.LocaleNameToLCID;

const fmt = std.fmt;

const ITfInputProcessorProfileMgr = profile.ITfInputProcessorProfileMgr;

const Guid = win32.zig.Guid;
pub fn registerProfile(
    dll_path: UTF16String,
    comptime description: UTF16StringLiteral,
    comptime guid: Guid,
    comptime guid_profile: Guid,
    comptime locale_id: u16,
) !void {
    const profile_manager = profile.createProfileManager() orelse {
        messageBox("Failed to create profile manager", "registerProfile", .Error);
        unreachable;
    };

    const icon_path = dll_path;

    _ = ITfInputProcessorProfileMgr.ITfInputProcessorProfileMgr_RegisterProfile(
        profile_manager,
        &guid,
        locale_id,
        &guid_profile,
        @ptrCast(description.ptr),
        @intCast(description.len),
        @ptrCast(icon_path.ptr),
        @intCast(icon_path.len),
        0,
        std.mem.zeroes(?win32.ui.text_services.HKL),
        0,
        @intFromBool(true),
        0,
    );

    // TODO: Wrap this in a convenience function

    messageBox("Profile registered!", "registerProfile", .Info);
}

pub fn unregisterProfile(
    comptime guid: Guid,
    comptime guid_profile: Guid,
    comptime locale_id: u16,
) !void {
    const profile_manager = profile.createProfileManager() orelse {
        messageBox("Failed to create profile manager", "unregisterProfile", .Error);
        return error.ProfileManagerCreationFailed;
    };

    _ = ITfInputProcessorProfileMgr.ITfInputProcessorProfileMgr_UnregisterProfile(
        profile_manager,
        &guid,
        locale_id,
        &guid_profile,
        0,
    );

    messageBox("Profile unregistered!", "unregisterProfile", .Info);
}

const category = @import("windows/category.zig");

const SUPPORTED_CATEGORIES: [7]Guid = .{
    category.GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
    category.GUID_TFCAT_TIPCAP_COMLESS,
    category.GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT,
    category.GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
    category.GUID_TFCAT_TIP_KEYBOARD,
    category.GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    category.GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
};

const ITfCategoryMgr = category.ITfCategoryMgr;

pub fn registerCategories(
    comptime guid: Guid,
) !void {
    messageBox("Registering categories", "registerCategories", .Info);
    const category_manager: *ITfCategoryMgr = category.createCategoryManager() orelse {
        messageBox("Failed to create category manager", "registerCategories", .Error);
        unreachable;
    };

    for (SUPPORTED_CATEGORIES) |guid_cat| {
        _ = ITfCategoryMgr.ITfCategoryMgr_RegisterCategory(
            category_manager,
            &guid,
            &guid_cat,
            &guid,
        );
    }
    messageBox("Categories registered", "registerCategories", .Info);
}

pub fn unregisterCategories(
    comptime guid: Guid,
) !void {
    messageBox("Unregistering categories", "unregisterCategories", .Info);
    const category_manager: *ITfCategoryMgr = category.createCategoryManager() orelse {
        messageBox("Failed to create category manager", "unregisterCategories", .Error);
        unreachable;
    };

    for (SUPPORTED_CATEGORIES) |guid_cat| {
        _ = ITfCategoryMgr.ITfCategoryMgr_UnregisterCategory(
            category_manager,
            &guid,
            &guid_cat,
            &guid,
        );
    }
    messageBox("Categories unregistered", "unregisterCategories", .Info);
}
