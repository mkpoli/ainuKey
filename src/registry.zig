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

pub fn registerServer(comptime service_name: UTF16StringLiteral, dll_path: UTF16String, comptime guid: UTF16StringLiteral) !void {
    messageBox("Registering server", "registerServer", .Info);
    const clsid_key: UTF16StringLiteral = CLSID ++ guid;
    const inproc_key = CLSID ++ guid ++ InprocServer32;

    messageBoxWZ(clsid_key, &[_:0]u16{}, .Info);
    messageBoxWZ(inproc_key, &[_:0]u16{}, .Info);
    try registry.createAndSetStringValue(HKEY_CLASSES_ROOT, clsid_key, null, service_name);
    try registry.createAndSetStringValue(HKEY_CLASSES_ROOT, inproc_key, null, dll_path);
    const threading_model_name = std.unicode.utf8ToUtf16LeStringLiteral("ThreadingModel");
    const threading_model_value = std.unicode.utf8ToUtf16LeStringLiteral("Apartment");
    try registry.createAndSetStringValue(HKEY_CLASSES_ROOT, inproc_key, threading_model_name, threading_model_value);

    messageBox("Server registered", "registerServer", .Info);
}

pub fn unregisterServer(comptime guid: UTF16StringLiteral) !void {
    const clsid_key = CLSID ++ guid;
    try registry.deleteTree(HKEY_CLASSES_ROOT, clsid_key);
}

// const LocaleNameToLCID = windows.LocaleNameToLCID;

// #[doc = "*Required features: `\"Win32_Globalization\"`*"]
// #[inline]
// pub unsafe fn LocaleNameToLCID<P0>(lpname: P0, dwflags: u32) -> u32
// where
//     P0: ::windows::core::IntoParam<::windows::core::PCWSTR>,
// {
//     ::windows_targets::link ! ( "kernel32.dll""system" fn LocaleNameToLCID ( lpname : ::windows::core::PCWSTR , dwflags : u32 ) -> u32 );
//     LocaleNameToLCID(lpname.into_param().abi(), dwflags)
// }

// int GetLocaleInfoEx(
//   [in, optional]  LPCWSTR lpLocaleName,
//   [in]            LCTYPE  LCType,
//   [out, optional] LPWSTR  lpLCData,
//   [in]            int     cchData
// );

const win32 = @import("win32");

const GetLocaleInfoEx = win32.globalization.GetLocaleInfoEx;
const LocaleNameToLCID = win32.globalization.LocaleNameToLCID;

const fmt = std.fmt;

const ITfInputProcessorProfiles = profile.ITfInputProcessorProfiles;
const Guid = win32.zig.Guid;
pub fn registerProfile(
    comptime language: UTF16StringLiteral,
    dll_path: UTF16String,
    comptime description: UTF16StringLiteral,
    comptime guid: Guid,
    comptime guid_profile: Guid,
) !void {
    const locale_id = LocaleNameToLCID(language, 0);
    // convert [:0]const u16

    var all_together: [100]u8 = undefined;
    var start: usize = 0;
    _ = &start;
    const all_together_slice = all_together[start..];

    const locale_id_debug = try fmt.bufPrint(all_together_slice, "LocaleID: {}", .{locale_id});

    // convert []u8 too

    messageBox(locale_id_debug, "registerProfile", .Info);

    var locale_info_buffer: [100:0]u16 = undefined;
    var locale_info_start: usize = 0;
    _ = &locale_info_start;
    const locale_info_slice = locale_info_buffer[locale_info_start..];

    const locale_info = GetLocaleInfoEx(language, 0, locale_info_slice, @as(i32, @intCast(locale_info_buffer.len)));

    const locale_info_debug = try fmt.bufPrint(all_together_slice, "LocaleInfo: {}", .{locale_info});

    messageBox(locale_info_debug, "registerProfile", .Info);

    const profiles = profile.createProfileManager() orelse {
        messageBox("Failed to create profile manager", "registerProfile", .Error);
        unreachable;
    };

    const icon_path = dll_path;

    _ = ITfInputProcessorProfiles.ITfInputProcessorProfiles_Register(
        profiles,
        &guid,
    );

    const locale_id_u16: u16 = @intCast(locale_id);

    _ = ITfInputProcessorProfiles.ITfInputProcessorProfiles_AddLanguageProfile(
        profiles,
        &guid,
        locale_id_u16,
        &guid_profile,
        @ptrCast(description.ptr),
        @intCast(description.len),
        @ptrCast(icon_path.ptr),
        @intCast(icon_path.len),
        0,
    );

    messageBox("Profile registered!", "registerProfile", .Info);
}

pub fn unregisterProfile(
    // comptime language: UTF16StringLiteral,
    comptime guid: Guid,
    // comptime guid_profile: Guid,
) !void {
    const profiles = profile.createProfileManager() orelse unreachable;

    // const locale_id = LocaleNameToLCID(language, 0);

    // const locale_id_u16: u16 = @intCast(locale_id);

    _ = ITfInputProcessorProfiles.ITfInputProcessorProfiles_Unregister(
        profiles,
        &guid,
    );

    messageBox("Profile unregistered!", "unregisterProfile", .Info);

    // _ = ITfInputProcessorProfiles.ITfInputProcessorProfiles_RemoveLanguageProfile(
    //     profiles,
    //     &guid,
    //     locale_id_u16,
    //     &guid_profile,
    // );
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
