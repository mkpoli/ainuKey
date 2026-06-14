//! Registration: COM in-proc server (registry writes), the input-processor
//! profile (`ITfInputProcessorProfileMgr`), and TSF categories
//! (`ITfCategoryMgr`). `register_all` / `unregister_all` wrap the three, with
//! COM initialized for the manager CoCreates.

use windows::core::{GUID, PCWSTR};
use windows::Win32::Foundation::{ERROR_SUCCESS, E_FAIL, MAX_PATH};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CLASSES_ROOT,
    KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::UI::Input::KeyboardAndMouse::HKL;
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, CLSID_TF_InputProcessorProfiles, ITfCategoryMgr,
    ITfInputProcessorProfileMgr, GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER, GUID_TFCAT_TIPCAP_COMLESS,
    GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT, GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT, GUID_TFCAT_TIPCAP_UIELEMENTENABLED, GUID_TFCAT_TIP_KEYBOARD,
};

use crate::dll_instance;
use crate::guids::{DISPLAY_DESCRIPTION, GUID_PROFILE, GUID_TEXT_SERVICE, SERVICE_NAME};

const CATEGORIES: [GUID; 7] = [
    GUID_TFCAT_TIP_KEYBOARD,
    GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
    GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT,
    GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
    GUID_TFCAT_TIPCAP_COMLESS,
    GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT,
];

/// The four desktop "keyboard transient" LCIDs.
///
/// Ainu (BCP-47 `ain`) has no assigned Windows LCID, so its input profile is
/// registered against all four transient slots. Once a user adds `ain` to their
/// language list, Windows assigns it one of these LCIDs; registering all four
/// up front (in the machine-wide `DllRegisterServer`) means whichever slot it
/// lands in already has a live profile. The per-user *enable* step
/// (`Set-WinUserLanguageList` + `InstallLayoutOrTip`) is done by the installer,
/// not here. See [MS-LCID] "Locale Names without LCIDs" and Keyman's
/// `RegisterTransientTips`.
const TRANSIENT_LANGIDS: [u16; 4] = [0x2000, 0x2400, 0x2800, 0x2C00];

/// Resolve the DLL's own absolute path, NUL-terminated, as UTF-16.
fn module_path_utf16() -> windows::core::Result<Vec<u16>> {
    let mut buf = [0u16; MAX_PATH as usize];
    // SAFETY: buf is a valid writable buffer; the module handle is captured.
    let len = unsafe { GetModuleFileNameW(Some(dll_instance()), &mut buf) };
    if len == 0 || len as usize >= buf.len() {
        return Err(E_FAIL.into());
    }
    let mut v = buf[..len as usize].to_vec();
    v.push(0);
    Ok(v)
}

/// Format a GUID as the canonical brace-wrapped string, e.g.
/// `{5ECECCEB-271D-4675-8EE5-8D129EF0CA08}`.
fn guid_braced(guid: &GUID) -> String {
    let d4 = guid.data4;
    format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        guid.data1, guid.data2, guid.data3, d4[0], d4[1], d4[2], d4[3], d4[4], d4[5], d4[6], d4[7]
    )
}

/// UTF-16, NUL-terminated.
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Bytes of a NUL-terminated wide string (for REG_SZ data).
fn reg_sz_bytes(wide_nul: &[u16]) -> Vec<u8> {
    wide_nul
        .iter()
        .flat_map(|&u| u.to_le_bytes())
        .collect::<Vec<u8>>()
}

struct RegKey(HKEY);

impl RegKey {
    /// Create (or open) a subkey under HKEY_CLASSES_ROOT for writing.
    fn create(subkey: &[u16]) -> windows::core::Result<Self> {
        let mut hkey = HKEY::default();
        // SAFETY: subkey is a valid NUL-terminated wide string; phkresult valid.
        let err = unsafe {
            RegCreateKeyExW(
                HKEY_CLASSES_ROOT,
                PCWSTR(subkey.as_ptr()),
                Some(0),
                PCWSTR::null(),
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                None,
                &mut hkey,
                None,
            )
        };
        if err != ERROR_SUCCESS {
            return Err(E_FAIL.into());
        }
        Ok(RegKey(hkey))
    }

    /// Set a REG_SZ value (`None` value name = the key's default value).
    fn set_string(&self, name: Option<&[u16]>, value_nul: &[u16]) -> windows::core::Result<()> {
        let data = reg_sz_bytes(value_nul);
        let name_pcwstr = match name {
            Some(n) => PCWSTR(n.as_ptr()),
            None => PCWSTR::null(),
        };
        // SAFETY: handle valid; data slice valid for its length.
        let err = unsafe { RegSetValueExW(self.0, name_pcwstr, Some(0), REG_SZ, Some(&data)) };
        if err != ERROR_SUCCESS {
            return Err(E_FAIL.into());
        }
        Ok(())
    }
}

impl Drop for RegKey {
    fn drop(&mut self) {
        // SAFETY: handle was created successfully.
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

// --- (a) COM in-proc server ------------------------------------------------

fn register_server() -> windows::core::Result<()> {
    let clsid_str = guid_braced(&GUID_TEXT_SERVICE);
    let clsid_key = wide(&format!("CLSID\\{}", clsid_str));
    let inproc_key = wide(&format!("CLSID\\{}\\InProcServer32", clsid_str));
    let dll_path = module_path_utf16()?;

    // HKCR\CLSID\{..} (default) = service name
    {
        let key = RegKey::create(&clsid_key)?;
        let name = wide_pcwstr_owned(SERVICE_NAME);
        key.set_string(None, &name)?;
    }
    // HKCR\CLSID\{..}\InProcServer32 (default) = dll path, ThreadingModel = Apartment
    {
        let key = RegKey::create(&inproc_key)?;
        key.set_string(None, &dll_path)?;
        let threading = wide("ThreadingModel");
        let apartment = wide("Apartment");
        key.set_string(Some(&threading), &apartment)?;
    }
    Ok(())
}

fn unregister_server() -> windows::core::Result<()> {
    let clsid_str = guid_braced(&GUID_TEXT_SERVICE);
    let clsid_key = wide(&format!("CLSID\\{}", clsid_str));
    // SAFETY: clsid_key is a valid NUL-terminated wide string.
    let err = unsafe { RegDeleteTreeW(HKEY_CLASSES_ROOT, PCWSTR(clsid_key.as_ptr())) };
    if err != ERROR_SUCCESS {
        // Best-effort: a missing key is fine.
        return Ok(());
    }
    Ok(())
}

/// Copy a `PCWSTR` static into an owned NUL-terminated `Vec<u16>`.
fn wide_pcwstr_owned(s: PCWSTR) -> Vec<u16> {
    // SAFETY: `s` points to a static NUL-terminated wide string (`w!` literal).
    unsafe {
        let mut len = 0usize;
        while *s.0.add(len) != 0 {
            len += 1;
        }
        let mut v = std::slice::from_raw_parts(s.0, len).to_vec();
        v.push(0);
        v
    }
}

// --- (b) Input-processor profile -------------------------------------------

fn register_profile() -> windows::core::Result<()> {
    // SAFETY: standard in-proc COM creation.
    let profiles: ITfInputProcessorProfileMgr =
        unsafe { CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)? };

    let name = wide_pcwstr_owned(DISPLAY_DESCRIPTION); // NUL-terminated
    let dll_path = module_path_utf16()?; // also the icon source (index 0 -> IDI_ICON)

    // Register the profile against every transient LCID Ainu might be assigned.
    // Not enabled by default — the installer enables the live one per user via
    // InstallLayoutOrTip.
    for &langid in &TRANSIENT_LANGIDS {
        // SAFETY: all pointers/slices valid for the call's duration.
        unsafe {
            profiles.RegisterProfile(
                &GUID_TEXT_SERVICE,
                langid,
                &GUID_PROFILE,
                &name[..name.len() - 1], // display name, without the trailing NUL
                &dll_path,               // icon file = the DLL itself
                0,                       // icon index -> IDI_ICON
                HKL(std::ptr::null_mut()),
                0,
                false, // enabled per-user via InstallLayoutOrTip, not by default
                0,
            )?;
        }
    }
    Ok(())
}

fn unregister_profile() -> windows::core::Result<()> {
    // SAFETY: standard in-proc COM creation.
    let profiles: ITfInputProcessorProfileMgr =
        unsafe { CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)? };
    for &langid in &TRANSIENT_LANGIDS {
        // SAFETY: profiles valid; GUID pointers valid. Best-effort per slot.
        unsafe {
            let _ = profiles.UnregisterProfile(&GUID_TEXT_SERVICE, langid, &GUID_PROFILE, 0);
        }
    }
    Ok(())
}

// --- (c) Categories --------------------------------------------------------

fn register_categories() -> windows::core::Result<()> {
    // SAFETY: standard in-proc COM creation.
    let catmgr: ITfCategoryMgr =
        unsafe { CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)? };
    for cat in CATEGORIES {
        // SAFETY: catmgr valid; GUID pointers valid.
        unsafe {
            catmgr.RegisterCategory(&GUID_TEXT_SERVICE, &cat, &GUID_TEXT_SERVICE)?;
        }
    }
    Ok(())
}

fn unregister_categories() -> windows::core::Result<()> {
    // SAFETY: standard in-proc COM creation.
    let catmgr: ITfCategoryMgr =
        unsafe { CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)? };
    for cat in CATEGORIES {
        // SAFETY: catmgr valid; GUID pointers valid.
        unsafe {
            let _ = catmgr.UnregisterCategory(&GUID_TEXT_SERVICE, &cat, &GUID_TEXT_SERVICE);
        }
    }
    Ok(())
}

// --- Orchestration ---------------------------------------------------------

/// RAII guard for COM initialization in this thread.
struct ComInit;

impl ComInit {
    fn new() -> Self {
        // SAFETY: initializing an STA on the registration thread.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        ComInit
    }
}

impl Drop for ComInit {
    fn drop(&mut self) {
        // SAFETY: balanced against the CoInitializeEx above.
        unsafe {
            CoUninitialize();
        }
    }
}

pub fn register_all() -> windows::core::Result<()> {
    let _com = ComInit::new();
    register_server()?;
    register_profile()?;
    register_categories()?;
    Ok(())
}

pub fn unregister_all() -> windows::core::Result<()> {
    let _com = ComInit::new();
    let _ = unregister_profile();
    let _ = unregister_categories();
    let _ = unregister_server();
    Ok(())
}
