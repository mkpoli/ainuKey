const std = @import("std");
const unicode = std.unicode;

const wintype = @import("windows/types.zig");
const UTF16String = wintype.UTF16String;
const UTF16StringLiteral = wintype.UTF16StringLiteral;

const win32 = @import("win32");
const Guid = win32.zig.Guid;
const toBraced = @import("windows/guid.zig").toBraced;

pub const NAME: UTF16StringLiteral = unicode.utf8ToUtf16LeStringLiteral("ainuKeyTextService");
pub const LANG: UTF16StringLiteral = unicode.utf8ToUtf16LeStringLiteral("ain");
pub const DESC: UTF16StringLiteral = unicode.utf8ToUtf16LeStringLiteral("ainuKey");

pub const GUID_TEXT_SERVICE = Guid.initString("5ECECCEB-271D-4675-8EE5-8D129EF0CA08");
pub const GUID_PROFILE = Guid.initString("5ECECCEC-271D-4675-8EE5-8D129EF0CA08");

const LANG_NEUTRAL = win32.system.system_services.LANG_NEUTRAL; // 0x00
const SUBLANG_NEUTRAL = win32.system.system_services.SUBLANG_CUSTOM_UNSPECIFIED; // 0x10
const LOCALE_CUSTOM_UNSPECIFIED: u32 = SUBLANG_NEUTRAL << 10 | LANG_NEUTRAL;

pub const LOCALE_ID: u32 = LOCALE_CUSTOM_UNSPECIFIED;
