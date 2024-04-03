const std = @import("std");

pub fn build(b: *std.Build) !void {
    const tool = b.addExecutable(.{ .name = "generate_time", .root_source_file = .{ .path = "tools/generate_time.zig" }, .target = b.host });

    const tool_step = b.addRunArtifact(tool);
    const output = tool_step.addOutputFileArg("version.zig");
    tool_step.addArg(try std.fmt.allocPrint(std.heap.page_allocator, "{d}", .{std.time.timestamp()}));
    // tool_step.has_side_effects = true;

    const win32 = b.addModule("win32", .{ .root_source_file = .{ .path = "lib/zigwin32/win32.zig" } });

    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const dll = b.addSharedLibrary(.{
        .name = "ainuKey",
        .root_source_file = .{ .path = "src/dllroot.zig" },
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });
    dll.root_module.addImport("win32", win32);
    dll.root_module.addAnonymousImport("version", .{ .root_source_file = output });
    dll.addWin32ResourceFile(.{ .file = .{
        .path = "assets/resources.rc",
    } });
    b.installArtifact(dll);

    const lib_unit_tests = b.addTest(.{
        .root_source_file = .{ .path = "src/dllroot.zig" },
        .target = target,
        .optimize = optimize,
    });

    const run_lib_unit_tests = b.addRunArtifact(lib_unit_tests);
    lib_unit_tests.root_module.addImport("win32", win32);

    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_lib_unit_tests.step);
}
