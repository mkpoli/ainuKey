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

pub fn registerServer(comptime service_name: UTF16StringLiteral, dll_path: UTF16String, comptime guid: Guid) error{ AccessDenied, Unexpected }!void {
    const clsid_key: UTF16StringLiteral = CLSID ++ toBraced(guid);
    const inproc_key = CLSID ++ toBraced(guid) ++ InprocServer32;
    // messageBoxWZ(clsid_key, &[_:0]u16{}, .Info);
    try registry.createAndSetStringValue(HKEY_CLASSES_ROOT, clsid_key, null, service_name);
    try registry.createAndSetStringValue(HKEY_CLASSES_ROOT, inproc_key, null, dll_path);
    const threading_model_name = std.unicode.utf8ToUtf16LeStringLiteral("ThreadingModel");
    const threading_model_value = std.unicode.utf8ToUtf16LeStringLiteral("Apartment");
    try registry.createAndSetStringValue(HKEY_CLASSES_ROOT, inproc_key, threading_model_name, threading_model_value);
}

pub fn unregisterServer(comptime guid: Guid) error{ AccessDenied, Unexpected }!void {
    const clsid_key = CLSID ++ toBraced(guid);
    try registry.deleteTree(HKEY_CLASSES_ROOT, clsid_key);
}

const win32 = @import("win32");

const ts = win32.ui.text_services;

const GetLocaleInfoEx = win32.globalization.GetLocaleInfoEx;
const LocaleNameToLCID = win32.globalization.LocaleNameToLCID;

const fmt = std.fmt;

const ITfInputProcessorProfileMgr = ts.ITfInputProcessorProfileMgr;

const Guid = win32.zig.Guid;
pub fn registerProfile(
    dll_path: UTF16String,
    comptime description: UTF16StringLiteral,
    comptime guid: Guid,
    comptime guid_profile: Guid,
    comptime locale_id: u16,
) error{Unexpected}!void {
    profile.registerProfile(
        guid,
        locale_id,
        guid_profile,
        description,
        dll_path,
        0,
        std.mem.zeroes(?win32.ui.text_services.HKL),
        0,
        true,
        0,
    ) catch |err| switch (err) {
        error.ProfileManagerCreationFailure => {
            messageBox("Failed to create profile manager", "registerProfile", .Error);
            return error.Unexpected;
        },
        error.ProfileRegistrationFailure => {
            messageBox("Failed to register profile", "registerProfile", .Error);
            return error.Unexpected;
        },
    };
}

pub fn unregisterProfile(
    comptime guid: Guid,
    comptime guid_profile: Guid,
    comptime locale_id: u16,
) error{Unexpected}!void {
    profile.unregisterProfile(
        guid,
        locale_id,
        guid_profile,
        0,
    ) catch |err| switch (err) {
        error.ProfileManagerCreationFailure => {
            messageBox("Failed to create profile manager", "unregisterProfile", .Error);
            return error.Unexpected;
        },
        error.ProfileUnregistrationFailure => {
            messageBox("Failed to unregister profile", "unregisterProfile", .Error);
            return error.Unexpected;
        },
    };
}

const SUPPORTED_CATEGORIES: [7]Guid = .{
    ts.GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
    ts.GUID_TFCAT_TIPCAP_COMLESS,
    ts.GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT,
    ts.GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
    ts.GUID_TFCAT_TIP_KEYBOARD,
    ts.GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    ts.GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
};

const ITfCategoryMgr = ts.ITfCategoryMgr;
const category = @import("windows/category.zig");

pub fn registerCategories(
    comptime guid: Guid,
) error{Unexpected}!void {
    category.registerCategories(guid, &SUPPORTED_CATEGORIES) catch |err| switch (err) {
        error.CategoryManagerCreationFailure => {
            messageBox("Failed to create category manager", "registerCategories", .Error);
            return error.Unexpected;
        },
        error.CategoryRegistrationFailure => {
            messageBox("Failed to register categories", "registerCategories", .Error);
            return error.Unexpected;
        },
    };
}

pub fn unregisterCategories(
    comptime guid: Guid,
) error{Unexpected}!void {
    category.unregisterCategories(guid, &SUPPORTED_CATEGORIES) catch |err| switch (err) {
        error.CategoryManagerCreationFailure => {
            messageBox("Failed to create category manager", "unregisterCategories", .Error);
            return error.Unexpected;
        },
        error.CategoryUnregistrationFailure => {
            messageBox("Failed to unregister categories", "unregisterCategories", .Error);
            return error.Unexpected;
        },
    };
}
