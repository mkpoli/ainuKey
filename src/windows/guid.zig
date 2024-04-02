const std = @import("std");
const Guid = @import("win32").zig.Guid;

pub fn toBraced(guid: Guid) [38]u8 {
    const bytes: [16]u8 = guid.Bytes;
    var result: [38]u8 = undefined;
    result[0] = '{';
    result[37] = '}';

    inline for (0..16) |i| {
        const byte = bytes[
            switch (i) {
                0...3 => 3 - i,
                4...5 => 5 - i + 4,
                6...7 => 7 - i + 6,
                else => i,
            }
        ];

        const high = (byte >> 4) & 0xF;
        const low = byte & 0xF;

        const pos_high = i * 2 + 1 + @intFromBool(i >= 4) + @intFromBool(i >= 6) + @intFromBool(i >= 8) + @intFromBool(i >= 10);
        const pos_low = pos_high + 1;

        result[pos_high] = "0123456789ABCDEF"[high];
        result[pos_low] = "0123456789ABCDEF"[low];

        switch (i) {
            3, 5, 7, 9 => {
                result[pos_low + 1] = '-';
            },
            else => {},
        }
    }

    return result;
}

test "GUID conversion" {
    const guid = Guid.initString("12345678-9ABC-DEF0-1234-56789ABCDEF0");
    const guid_str = toBraced(guid);

    const expected: []const u8 = "{12345678-9ABC-DEF0-1234-56789ABCDEF0}";
    const guid_slice: []const u8 = &guid_str;

    try std.testing.expectEqualSlices(u8, guid_slice, expected);
}
