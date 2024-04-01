const std = @import("std");
const fs = std.fs;
const Build = std.Build;

pub fn build(b: *Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const dll = b.addSharedLibrary(.{
        .name = "ainuKey",
        .root_source_file = .{ .path = "src/dllroot.zig" },
        .target = target,
        .optimize = optimize,
        .link_libc = true,
    });

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

    const test_step = b.step("test", "Run unit tests");
    test_step.dependOn(&run_lib_unit_tests.step);
}
