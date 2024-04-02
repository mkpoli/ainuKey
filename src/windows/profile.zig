const std = @import("std");

const windows = std.os.windows;
const WINAPI = windows.WINAPI;
const HRESULT = windows.HRESULT;

const com = @import("com.zig");
const Guid = win32.zig.Guid;
const CoCreateInstance = com.CoCreateInstance;
pub const CLSID_TF_InputProcessorProfiles = com.CLSID_TF_InputProcessorProfiles;
pub const IID_TF_InputProcessorProfiles = com.IID_ITfInputProcessorProfiles;

const S_OK = windows.S_OK;
const E_NOINTERFACE = windows.E_NOINTERFACE;
const E_OUTOFMEMORY = windows.E_OUTOFMEMORY;

pub fn createInstanceInproc(T: type, clsid: Guid) ?*T {
    var result: ?*T = null;
    _ = CoCreateInstance(&clsid, null, com.CLSCTX_INPROC_SERVER, &T.IID, @ptrCast(&result));
    return result;
}

const win32 = @import("win32");

const IUnknown = win32.system.com.IUnknown;

pub const ITfInputProcessorProfiles = extern struct {
    pub const IID = com.IID_ITfInputProcessorProfiles;

    pub const VTable = extern struct {
        base: IUnknown.VTable,
        Register: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfInputProcessorProfiles, rclsid: *const Guid) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfInputProcessorProfiles, rclsid: *const Guid) callconv(WINAPI) HRESULT,
        },
        Unregister: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfInputProcessorProfiles, rclsid: *const Guid) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfInputProcessorProfiles, rclsid: *const Guid) callconv(WINAPI) HRESULT,
        },
        AddLanguageProfile: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfInputProcessorProfiles, rclsid: *const Guid, langid: u16, guidprofile: *const Guid, pchDesc: ?*const u16, cchDesc: u32, pchiconfile: ?*const u16, cchfile: u32, uiconindex: u32) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfInputProcessorProfiles, rclsid: *const Guid, langid: u16, guidProfile: *const Guid, pchDesc: ?*const u16, cchDesc: u32, pchiconfile: ?*const u16, cchfile: u32, uiconindex: u32) callconv(WINAPI) HRESULT,
        },
        RemoveLanguageProfile: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfInputProcessorProfiles, rclsid: *const Guid, langid: u16, guidProfile: *const Guid) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfInputProcessorProfiles, rclsid: *const Guid, langid: u16, guidProfile: *const Guid) callconv(WINAPI) HRESULT,
        },
        // Additional methods follow the same pattern
    };
    vtable: *const VTable,

    pub fn MethodMixin(comptime T: type) type {
        return struct {
            pub usingnamespace IUnknown.MethodMixin(T);

            pub inline fn ITfInputProcessorProfiles_Register(self: *const T, rclsid: *const Guid) HRESULT {
                return @as(*const ITfInputProcessorProfiles.VTable, @ptrCast(self.vtable)).Register(@as(*const ITfInputProcessorProfiles, @ptrCast(self)), rclsid);
            }

            pub inline fn ITfInputProcessorProfiles_Unregister(self: *const T, rclsid: *const Guid) HRESULT {
                return @as(*const ITfInputProcessorProfiles.VTable, @ptrCast(self.vtable)).Unregister(@as(*const ITfInputProcessorProfiles, @ptrCast(self)), rclsid);
            }

            pub inline fn ITfInputProcessorProfiles_AddLanguageProfile(self: *const T, rclsid: *const Guid, langid: u16, guidProfile: *const Guid, pchDesc: ?*const u16, cchDesc: u32, pchiconfile: ?*const u16, cchfile: u32, uiconindex: u32) HRESULT {
                return @as(*const ITfInputProcessorProfiles.VTable, @ptrCast(self.vtable)).AddLanguageProfile(@as(*const ITfInputProcessorProfiles, @ptrCast(self)), rclsid, langid, guidProfile, pchDesc, cchDesc, pchiconfile, cchfile, uiconindex);
            }

            pub inline fn ITfInputProcessorProfiles_RemoveLanguageProfile(self: *const T, rclsid: *const Guid, langid: u16, guidProfile: *const Guid) HRESULT {
                return @as(*const ITfInputProcessorProfiles.VTable, @ptrCast(self.vtable)).RemoveLanguageProfile(@as(*const ITfInputProcessorProfiles, @ptrCast(self)), rclsid, langid, guidProfile);
            }
        };
    }
    pub usingnamespace MethodMixin(@This());
};

pub fn createProfileManager() ?*ITfInputProcessorProfiles {
    return com.createInstanceInproc(ITfInputProcessorProfiles, CLSID_TF_InputProcessorProfiles);
}
