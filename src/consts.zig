const std = @import("std");
const unicode = std.unicode;

const wintype = @import("windows/types.zig");
const UTF16String = wintype.UTF16String;
const UTF16StringLiteral = wintype.UTF16StringLiteral;

pub const GUID: UTF16StringLiteral = unicode.utf8ToUtf16LeStringLiteral("{5ECECCEB-271D-4675-8EE5-8D129EF0CA08}");
pub const NAME: UTF16StringLiteral = unicode.utf8ToUtf16LeStringLiteral("ainuKeyTextService");
pub const LANG: UTF16StringLiteral = unicode.utf8ToUtf16LeStringLiteral("ain");
