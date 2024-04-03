const std = @import("std");
pub fn main() !void {
    var arena_state = std.heap.ArenaAllocator.init(std.heap.page_allocator);
    defer arena_state.deinit();
    const arena = arena_state.allocator();

    const args = try std.process.argsAlloc(arena);
    defer std.process.argsFree(arena, args);

    if (args.len < 2) return error.@"Usage: version.zig <version_file_path>\n";

    // con
    const output_file_path = args[1];

    const jst_offset = 60 * 60 * 9;

    const now = std.time.epoch.EpochSeconds{ .secs = @intCast(std.time.timestamp() + jst_offset) };
    const month_day = now.getEpochDay().calculateYearDay().calculateMonthDay();
    const day_seconds = now.getDaySeconds();
    const month = month_day.month.numeric();
    const day = month_day.day_index + 1;
    const hour = day_seconds.getHoursIntoDay();
    const minute = day_seconds.getMinutesIntoHour();

    var version_file = try std.fs.cwd().createFile(output_file_path, .{});
    defer version_file.close();

    const version = try std.fmt.allocPrint(std.heap.page_allocator, "{d:0>2}/{d:0>2} {d:0>2}:{d:0>2}", .{ month, day, hour, minute });
    const module = try std.fmt.allocPrint(std.heap.page_allocator, "pub const VERSION = \"{s}\";", .{version});
    try version_file.writeAll(module);

    return std.process.cleanExit();
}
