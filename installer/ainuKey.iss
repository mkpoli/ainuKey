; Inno Setup script for ainuKey — installs and registers the TSF IME DLL.
;
; Build:
;   ISCC.exe /DMyAppVersion=0.1.0 installer\ainuKey.iss
; Output:
;   ../ainuKey-<version>-x86_64-windows-setup.exe  (repo root)

#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif
#define MyAppName "ainuKey"
#define MyAppPublisher "mkpoli"
#define MyAppURL "https://github.com/mkpoli/ainuKey"

[Setup]
; Stable application id (distinct from the COM/TSF GUIDs).
AppId={{5ECECCEE-271D-4675-8EE5-8D129EF0CA08}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
DefaultDirName={autopf}\ainuKey
DisableProgramGroupPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
OutputDir=..
OutputBaseFilename=ainuKey-{#MyAppVersion}-x86_64-windows-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
LicenseFile=..\LICENSE
UninstallDisplayName={#MyAppName}
UninstallDisplayIcon={app}\ainukey.dll

[Files]
; `regserver` calls DllRegisterServer on install and DllUnregisterServer on
; uninstall. In 64-bit install mode the 64-bit DLL is registered correctly.
; `regserver` calls DllRegisterServer, which registers the profile under the
; Japanese langid enabled by default — so no per-user step is needed.
Source: "..\target\x86_64-pc-windows-msvc\release\ainukey.dll"; DestDir: "{app}"; Flags: ignoreversion regserver
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE";   DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Start-menu shortcut that opens the settings dialog via the DLL's ShowSettings
; export. This is the reachable way to settings on Windows 11, where the floating
; language bar — and therefore the langbar "設定 / Settings…" menu — is hidden by
; default. {sys}\rundll32.exe is 64-bit here (ArchitecturesInstallIn64BitMode),
; so it can load the 64-bit DLL.
Name: "{autoprograms}\ainuKey Settings"; Filename: "{sys}\rundll32.exe"; Parameters: """{app}\ainukey.dll"",ShowSettings"; WorkingDir: "{app}"; Comment: "ainuKey — 設定 / Settings"

[Messages]
; Shown on the final page.
FinishedLabel=ainuKey was installed. Switch input (Win+Space) to ainuKey (listed under Japanese) and type romaji.
