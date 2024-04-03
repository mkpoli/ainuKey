//! This module provides IClassFactory implementation for Text Service

const std = @import("std");
const Allocator = std.mem.Allocator;

const win32 = @import("win32");

const WINAPI = std.os.windows.WINAPI;

const S_OK = win32.foundation.S_OK;
const E_NOINTERFACE = win32.foundation.E_NOINTERFACE;
const E_POINTER = win32.foundation.E_POINTER;

const IUnknown = win32.system.com.IUnknown;
const IClassFactory = win32.system.com.IClassFactory;

const Guid = win32.zig.Guid;
const HRESULT = win32.foundation.HRESULT;

const CLASS_E_NOAGGREGATION = win32.foundation.CLASS_E_NOAGGREGATION;

const IID_ClassFactory = win32.system.com.IID_IClassFactory;

const dllroot = @import("dllroot.zig");

const messageBox = @import("windows/debug.zig").messageBox;
const mBAP = @import("windows/debug.zig").messageBoxAllocPrint;
const global_allocator = std.heap.c_allocator;

pub const ClassFactory = extern struct {
    const Self = @This();
    const VTable = extern struct {
        base: IClassFactory.VTable,
    };
    const CreateFn = *const fn (
        riid: ?*const Guid,
        ppvObject: ?*?*anyopaque,
    ) callconv(WINAPI) HRESULT;

    vtable: *const VTable,
    create_fn: CreateFn,
    ref: usize,

    pub fn Unknown_QueryInterface(
        self: *const IUnknown,
        riid: ?*const Guid,
        ppvObject: ?*?*anyopaque,
    ) callconv(WINAPI) HRESULT {
        if (!std.meta.eql(riid.?, IID_ClassFactory)) {
            ppvObject.?.* = null;
            return E_NOINTERFACE;
        }

        ppvObject.?.* = @constCast(self);

        _ = self.vtable.AddRef(self);

        return S_OK;
    }

    pub fn Unknown_AddRef(_self: *const IUnknown) callconv(WINAPI) u32 {
        const self: *Self = @as(*Self, @constCast(@ptrCast(_self)));
        self.ref += 1;
        return @intCast(self.ref);
    }

    pub fn Unknown_Release(_self: *const IUnknown) callconv(WINAPI) u32 {
        const self: *Self = @as(*Self, @constCast(@ptrCast(_self)));
        self.ref -= 1;

        if (self.ref == 0) {
            global_allocator.destroy(self);
            _ = @atomicRmw(i32, &dllroot.ref_count, .Sub, 1, .monotonic);
            return 0;
        }

        return @intCast(self.ref);
    }

    pub fn ClassFactory_CreateInstance(_self: *const IClassFactory, pUnkOuter: ?*IUnknown, riid: ?*const Guid, ppvObject: ?*?*anyopaque) callconv(WINAPI) HRESULT {
        messageBox("CreateInstance()", "ClassFactory", .Info);

        const self: *Self = @as(*Self, @constCast(@ptrCast(_self)));

        if (pUnkOuter != null) {
            return CLASS_E_NOAGGREGATION;
        }

        return self.create_fn(riid, ppvObject);
    }

    pub fn ClassFactory_LockServer(self: *const IClassFactory, fLock: i32) callconv(WINAPI) HRESULT {
        messageBox("LockServer()", "ClassFactory", .Info);
        _ = self;

        if (fLock != 0) {
            _ = @atomicRmw(i32, &dllroot.ref_lock, .Add, 1, .monotonic);
        } else {
            _ = @atomicRmw(i32, &dllroot.ref_lock, .Sub, 1, .monotonic);
        }
        return S_OK;
    }

    pub const vtable_impl: Self.VTable = .{ .base = .{
        .base = .{
            .QueryInterface = Unknown_QueryInterface,
            .AddRef = Unknown_AddRef,
            .Release = Unknown_Release,
        },
        .CreateInstance = ClassFactory_CreateInstance,
        .LockServer = ClassFactory_LockServer,
    } };

    pub fn create(
        allocator: Allocator,
        create_fn: CreateFn,
        riid: ?*const Guid,
        ppvObject: ?*?*anyopaque,
    ) error{ NoInterface, NullPointer, OutOfMemory, Unexpected }!void {
        messageBox("before creation", "ClassFactory.create()", .Info);
        var obj = try allocator.create(Self);
        messageBox("allocated", "ClassFactory.create()", .Info);

        obj.vtable = &Self.vtable_impl;
        obj.create_fn = create_fn;
        obj.ref = 1;

        _ = riid;

        const result = obj.vtable.base.base.QueryInterface(@ptrCast(obj),

        // riid
        // TODO:
        IID_ClassFactory, ppvObject); // IUnknown

        _ = obj.vtable.base.base.Release(@ptrCast(obj));

        _ = @atomicRmw(i32, &dllroot.ref_count, .Add, 1, .monotonic);

        switch (result) {
            S_OK => {
                messageBox("QueryInterface() succeeded", "ClassFactory.create()", .Info);
            },
            E_NOINTERFACE => {
                return error.NoInterface;
            },
            E_POINTER => {
                return error.NullPointer;
            },
            else => {
                mBAP("QueryInterface() failed: {X}", .{result}, "ClassFactory.create()", .Error);
                return error.Unexpected;
            },
        }
        // std.builtin.AtomicOrder.monotonic
    }
};
