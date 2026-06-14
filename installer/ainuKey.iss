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
Source: "..\target\x86_64-pc-windows-msvc\release\ainukey.dll"; DestDir: "{app}"; Flags: ignoreversion regserver
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE";   DestDir: "{app}"; Flags: ignoreversion

[Messages]
; Shown on the final page.
FinishedLabel=ainuKey was installed. Switch input method (Win+Space) to "ainuKey" and type romaji. If it does not appear, add Japanese under Settings > Time & language > Language.
