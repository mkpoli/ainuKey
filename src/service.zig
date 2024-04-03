const messageBox = @import("windows/debug.zig").messageBox;

const std = @import("std");

const Allocator = std.mem.Allocator;
const WINAPI = std.os.windows.WINAPI;

const win32 = @import("win32");

const IUnknown = win32.system.com.IUnknown;
const Guid = win32.zig.Guid;

const HRESULT = win32.foundation.HRESULT;
const S_OK = win32.foundation.S_OK;
const E_OUTOFMEMORY = win32.foundation.E_OUTOFMEMORY;
const E_NOINTERFACE = win32.foundation.E_NOINTERFACE;
const E_POINTER = win32.foundation.E_POINTER;

const ITfTextInputProcessor = win32.ui.text_services.ITfTextInputProcessor;
const IID_ITfTextInputProcessor = win32.ui.text_services.IID_ITfTextInputProcessor;

const ITfThreadMgr = win32.ui.text_services.ITfThreadMgr;

const CLASS_E_NOAGGREGATION = win32.foundation.CLASS_E_NOAGGREGATION;

pub var ref_count: i32 = 0;
const mBAP = @import("windows/debug.zig").messageBoxAllocPrint;
pub const TextService = extern struct {
    const Self = @This();
    const VTable = extern struct {
        base: ITfTextInputProcessor.VTable,
    };
    const CreateFn = *const fn (
        riid: ?*const Guid,
        ppvObject: ?*?*anyopaque,
    ) callconv(WINAPI) HRESULT;

    vtable: *const VTable,
    ref: usize,

    pub fn Unknown_QueryInterface(
        self: *const IUnknown,
        riid: ?*const Guid,
        ppvObject: ?*?*anyopaque,
    ) callconv(WINAPI) HRESULT {
        if (!std.meta.eql(riid.?, IID_ITfTextInputProcessor)) {
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
            std.heap.c_allocator.destroy(self);
            _ = @atomicRmw(i32, &ref_count, .Sub, 1, .monotonic);
            return 0;
        }

        return @intCast(self.ref);
    }

    pub fn ITfTextInputProcessor_Activate(
        self: *const ITfTextInputProcessor,
        ptim: ?*ITfThreadMgr,
        tid: u32,
    ) callconv(WINAPI) HRESULT {
        messageBox("Activate()", "TextService", .Info);
        _ = self;
        _ = tid;
        _ = ptim;
        return S_OK;
    }

    pub fn ITfTextInputProcessor_Deactivate(self: *const ITfTextInputProcessor) callconv(WINAPI) HRESULT {
        messageBox("Deactivate()", "TextService", .Info);
        _ = self;
        return S_OK;
    }

    pub const vtable_impl: Self.VTable = .{ .base = .{
        .base = .{
            .QueryInterface = Unknown_QueryInterface,
            .AddRef = Unknown_AddRef,
            .Release = Unknown_Release,
        },
        .Activate = ITfTextInputProcessor_Activate,
        .Deactivate = ITfTextInputProcessor_Deactivate,
    } };

    pub fn create(
        riid: ?*const Guid,
        ppv: ?*?*anyopaque,
    ) callconv(WINAPI) HRESULT {
        messageBox("TextService created", "TextService.create()", .Info);

        _ = riid;
        var obj = std.heap.c_allocator.create(Self) catch {
            messageBox("Out of memory", "TextService.create()", .Error);
            return E_OUTOFMEMORY;
        };

        obj.* = .{ .vtable = &Self.vtable_impl, .ref = 1 };

        const result = obj.vtable.base.base.QueryInterface(@ptrCast(obj), IID_ITfTextInputProcessor, ppv); // IUnknown

        _ = obj.vtable.base.base.Release(@ptrCast(obj));

        _ = @atomicRmw(i32, &ref_count, .Add, 1, .monotonic);

        switch (result) {
            S_OK => {
                messageBox("QueryInterface() succeeded", "TextService.create()", .Info);
            },
            E_NOINTERFACE => {
                messageBox("QueryInterface() failed: E_NOINTERFACE", "TextService.create()", .Error);
            },
            E_POINTER => {
                messageBox("QueryInterface() failed: E_POINTER", "TextService.create()", .Error);
            },
            else => {
                mBAP("QueryInterface() failed: {X}", .{result}, "TextService.create()", .Error);
            },
        }
        return result;
    }
};
