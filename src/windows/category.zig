const std = @import("std");

const windows = std.os.windows;
const WINAPI = windows.WINAPI;
const HRESULT = windows.HRESULT;

const com = @import("com.zig");
const Guid = win32.zig.Guid;
const CoCreateInstance = com.CoCreateInstance;
pub const CLSID_TF_InputProcessorProfiles = com.CLSID_TF_InputProcessorProfiles;
pub const IID_TF_InputProcessorProfiles = com.IID_ITfInputProcessorProfiles;

const win32 = @import("win32");

const IUnknown = win32.system.com.IUnknown;

pub const ITfCategoryMgr = extern struct {
    pub const IID = com.IID_ITfCategoryMgr;

    pub const VTable = extern struct {
        base: IUnknown.VTable,
        RegisterCategory: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfCategoryMgr, rclsid: *const Guid, rcatid: *const Guid, rguid: *const Guid) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfCategoryMgr, rclsid: *const Guid, rcatid: *const Guid, rguid: *const Guid) callconv(WINAPI) HRESULT,
        },
        UnregisterCategory: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfCategoryMgr, rclsid: *const Guid, rcatid: *const Guid, rguid: *const Guid) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfCategoryMgr, rclsid: *const Guid, rcatid: *const Guid, rguid: *const Guid) callconv(WINAPI) HRESULT,
        },
        EnumCategoriesInItem: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfCategoryMgr, rguid: *const Guid) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfCategoryMgr, rguid: *const Guid) callconv(WINAPI) HRESULT,
        },
        EnumItemsInCategory: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfCategoryMgr, rcatid: *const Guid) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfCategoryMgr, rcatid: *const Guid) callconv(WINAPI) HRESULT,
        },
        FindClosestCategory: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfCategoryMgr, rguid: *const Guid, pcatid: *Guid, ppcatidlist: *const *const Guid) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfCategoryMgr, rguid: *const Guid, pcatid: *Guid, ppcatidlist: *const *const Guid) callconv(WINAPI) HRESULT,
        },
        RegisterGUIDDescription: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfCategoryMgr, rclsid: *const Guid, rguid: *const Guid, pchdesc: ?*const u16) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfCategoryMgr, rclsid: *const Guid, rguid: *const Guid, pchdesc: ?*const u16) callconv(WINAPI) HRESULT,
        },
        RemoveGUIDDescription: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfCategoryMgr, rclsid: *const Guid, rguid: *const Guid) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfCategoryMgr, rclsid: *const Guid, rguid: *const Guid) callconv(WINAPI) HRESULT,
        },
        GetGUIDDescription: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfCategoryMgr, rguid: *const Guid) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfCategoryMgr, rguid: *const Guid) callconv(WINAPI) HRESULT,
        },
        RegisterGUIDDWORD: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfCategoryMgr, rclsid: *const Guid, rguid: *const Guid, dw: u32) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfCategoryMgr, rclsid: *const Guid, rguid: *const Guid, dw: u32) callconv(WINAPI) HRESULT,
        },
        UnregisterGUIDDWORD: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfCategoryMgr, rclsid: *const Guid, rguid: *const Guid) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfCategoryMgr, rclsid: *const Guid, rguid: *const Guid) callconv(WINAPI) HRESULT,
        },
        GetGUIDDWORD: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfCategoryMgr, rguid: *const Guid) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfCategoryMgr, rguid: *const Guid) callconv(WINAPI) HRESULT,
        },
        RegisterGUID: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfCategoryMgr, rguid: *const Guid) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfCategoryMgr, rguid: *const Guid) callconv(WINAPI) HRESULT,
        },
        GetGUID: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfCategoryMgr, guidatom: u32) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfCategoryMgr, guidatom: u32) callconv(WINAPI) HRESULT,
        },
        IsEqualTfGuidAtom: switch (@import("builtin").zig_backend) {
            .stage1 => fn (self: *const ITfCategoryMgr, guidatom: u32, rguid: *const Guid) callconv(WINAPI) HRESULT,
            else => *const fn (self: *const ITfCategoryMgr, guidatom: u32, rguid: *const Guid) callconv(WINAPI) HRESULT,
        },
    };

    vtable: *const VTable,

    pub fn MethodMixin(comptime T: type) type {
        return struct {
            pub usingnamespace IUnknown.MethodMixin(T);

            pub inline fn ITfCategoryMgr_RegisterCategory(self: *const T, rclsid: *const Guid, rcatid: *const Guid, rguid: *const Guid) HRESULT {
                return @as(*const ITfCategoryMgr.VTable, @ptrCast(self.vtable)).RegisterCategory(@as(*const ITfCategoryMgr, @ptrCast(self)), rclsid, rcatid, rguid);
            }

            pub inline fn ITfCategoryMgr_UnregisterCategory(self: *const T, rclsid: *const Guid, rcatid: *const Guid, rguid: *const Guid) HRESULT {
                return @as(*const ITfCategoryMgr.VTable, @ptrCast(self.vtable)).UnregisterCategory(@as(*const ITfCategoryMgr, @ptrCast(self)), rclsid, rcatid, rguid);
            }

            pub inline fn ITfCategoryMgr_EnumCategoriesInItem(self: *const T, rguid: *const Guid) HRESULT {
                return @as(*const ITfCategoryMgr.VTable, @ptrCast(self.vtable)).EnumCategoriesInItem(@as(*const ITfCategoryMgr, @ptrCast(self)), rguid);
            }

            pub inline fn ITfCategoryMgr_EnumItemsInCategory(self: *const T, rcatid: *const Guid) HRESULT {
                return @as(*const ITfCategoryMgr.VTable, @ptrCast(self.vtable)).EnumItemsInCategory(@as(*const ITfCategoryMgr, @ptrCast(self)), rcatid);
            }

            pub inline fn ITfCategoryMgr_FindClosestCategory(self: *const T, rguid: *const Guid, pcatid: *Guid, ppcatidlist: *const *const Guid) HRESULT {
                return @as(*const ITfCategoryMgr.VTable, @ptrCast(self.vtable)).FindClosestCategory(@as(*const ITfCategoryMgr, @ptrCast(self)), rguid, pcatid, ppcatidlist);
            }

            pub inline fn ITfCategoryMgr_RegisterGUIDDescription(self: *const T, rclsid: *const Guid, rguid: *const Guid, pchdesc: ?*const u16) HRESULT {
                return @as(*const ITfCategoryMgr.VTable, @ptrCast(self.vtable)).RegisterGUIDDescription(@as(*const ITfCategoryMgr, @ptrCast(self)), rclsid, rguid, pchdesc);
            }

            pub inline fn ITfCategoryMgr_RemoveGUIDDescription(self: *const T, rclsid: *const Guid, rguid: *const Guid) HRESULT {
                return @as(*const ITfCategoryMgr.VTable, @ptrCast(self.vtable)).RemoveGUIDDescription(@as(*const ITfCategoryMgr, @ptrCast(self)), rclsid, rguid);
            }

            pub inline fn ITfCategoryMgr_GetGUIDDescription(self: *const T, rguid: *const Guid) HRESULT {
                return @as(*const ITfCategoryMgr.VTable, @ptrCast(self.vtable)).GetGUIDDescription(@as(*const ITfCategoryMgr, @ptrCast(self)), rguid);
            }

            pub inline fn ITfCategoryMgr_RegisterGUIDDWORD(self: *const T, rclsid: *const Guid, rguid: *const Guid, dw: u32) HRESULT {
                return @as(*const ITfCategoryMgr.VTable, @ptrCast(self.vtable)).RegisterGUIDDWORD(@as(*const ITfCategoryMgr, @ptrCast(self)), rclsid, rguid, dw);
            }

            pub inline fn ITfCategoryMgr_UnregisterGUIDDWORD(self: *const T, rclsid: *const Guid, rguid: *const Guid) HRESULT {
                return @as(*const ITfCategoryMgr.VTable, @ptrCast(self.vtable)).UnregisterGUIDDWORD(@as(*const ITfCategoryMgr, @ptrCast(self)), rclsid, rguid);
            }

            pub inline fn ITfCategoryMgr_GetGUIDDWORD(self: *const T, rguid: *const Guid) HRESULT {
                return @as(*const ITfCategoryMgr.VTable, @ptrCast(self.vtable)).GetGUIDDWORD(@as(*const ITfCategoryMgr, @ptrCast(self)), rguid);
            }

            pub inline fn ITfCategoryMgr_RegisterGUID(self: *const T, rguid: *const Guid) HRESULT {
                return @as(*const ITfCategoryMgr.VTable, @ptrCast(self.vtable)).RegisterGUID(@as(*const ITfCategoryMgr, @ptrCast(self)), rguid);
            }

            pub inline fn ITfCategoryMgr_GetGUID(self: *const T, guidatom: u32) HRESULT {
                return @as(*const ITfCategoryMgr.VTable, @ptrCast(self.vtable)).GetGUID(@as(*const ITfCategoryMgr, @ptrCast(self)), guidatom);
            }

            pub inline fn ITfCategoryMgr_IsEqualTfGuidAtom(self: *const T, guidatom: u32, rguid: *const Guid) HRESULT {
                return @as(*const ITfCategoryMgr.VTable, @ptrCast(self.vtable)).IsEqualTfGuidAtom(@as(*const ITfCategoryMgr, @ptrCast(self)), guidatom, rguid);
            }
        };
    }
    pub usingnamespace MethodMixin(@This());
};

pub fn createCategoryManager() ?*ITfCategoryMgr {
    return com.createInstanceInproc(ITfCategoryMgr, com.CLSID_TF_CategoryMgr);
}

pub const GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER: Guid = Guid.initString("046b8c80-1647-40f7-9b21-b93b81aabc1b");
pub const GUID_TFCAT_TIPCAP_COMLESS: Guid = Guid.initString("364215d9-75bc-11d7-a6ef-00065b84435c");
pub const GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT: Guid = Guid.initString("ccf05dd7-4a87-11d7-a6e2-00065b84435c");
pub const GUID_TFCAT_TIPCAP_UIELEMENTENABLED: Guid = Guid.initString("49d2f9cf-1f5e-11d7-a6d3-00065b84435c");
pub const GUID_TFCAT_TIP_KEYBOARD: Guid = Guid.initString("34745c63-b2f0-4784-8b67-5e12c8701a31");
pub const GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT: Guid = Guid.initString("13a016df-560b-46cd-947a-4c3af1e0e35d");
pub const GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT: Guid = Guid.initString("25504fb4-7bab-4bc1-9c69-cf81890f0ef5");
