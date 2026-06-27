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
; Always offer the language picker at startup (more than one language below).
ShowLanguageDialog=yes

[Languages]
; English / Japanese / Russian ship with Inno Setup. Chinese (Simplified) and
; Korean are NOT in the released ISCC's Languages folder (Chinese isn't in any
; 6.7.x release yet; verified empirically — the build fails to open them), so
; both are vendored under installer/i18n/ (community translations, UTF-8 BOM).
; Ainu (Aynu itak) is a separate attested-only locale built in its own PR.
Name: "english";           MessagesFile: "compiler:Default.isl"
Name: "japanese";          MessagesFile: "compiler:Languages\Japanese.isl"
Name: "russian";           MessagesFile: "compiler:Languages\Russian.isl"
Name: "chinesesimplified"; MessagesFile: "i18n\ChineseSimplified.isl"
Name: "korean";            MessagesFile: "i18n\Korean.isl"

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
; Final-page instruction, per language. The technical step (switch input + type
; romaji) is the same everywhere; only the wording is localized.
english.FinishedLabel=ainuKey was installed. Switch input (Win+Space) to ainuKey (listed under Japanese) and type romaji.
japanese.FinishedLabel=ainuKeyをインストールしました。入力切替（Win+Space）でainuKey（「日本語」の下に表示されます）を選び、ローマ字で入力してください。
russian.FinishedLabel=ainuKey установлен. Переключите ввод (Win+Space) на ainuKey (в разделе «Японский») и печатайте латиницей (ромадзи).
chinesesimplified.FinishedLabel=ainuKey 已安装。请用 Win+Space 将输入法切换到 ainuKey（位于“日语”下方），然后输入罗马字。
korean.FinishedLabel=ainuKey가 설치되었습니다. 입력 전환(Win+Space)에서 ainuKey(“일본어” 아래에 표시됨)를 선택하고 로마자로 입력하세요.
