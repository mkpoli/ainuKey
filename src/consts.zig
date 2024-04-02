const std = @import("std");
const unicode = std.unicode;

const wintype = @import("windows/types.zig");
const UTF16String = wintype.UTF16String;
const UTF16StringLiteral = wintype.UTF16StringLiteral;

const win32 = @import("win32");
const Guid = win32.zig.Guid;

pub const GUID: UTF16StringLiteral = unicode.utf8ToUtf16LeStringLiteral("{5ECECCEB-271D-4675-8EE5-8D129EF0CA08}");
pub const NAME: UTF16StringLiteral = unicode.utf8ToUtf16LeStringLiteral("ainuKeyTextService");
pub const LANG: UTF16StringLiteral = unicode.utf8ToUtf16LeStringLiteral("ain");
pub const DESC: UTF16StringLiteral = unicode.utf8ToUtf16LeStringLiteral("ainuKey");

pub const GUID_GUID = Guid.initString("5ECECCEB-271D-4675-8EE5-8D129EF0CA08");

pub const GUID_PROFILE: UTF16StringLiteral = unicode.utf8ToUtf16LeStringLiteral("{5ECECCEC-271D-4675-8EE5-8D129EF0CA08}");
pub const GUID_PROFILE_GUID = Guid.initString("5ECECCEC-271D-4675-8EE5-8D129EF0CA08");
