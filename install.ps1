<#
  install.ps1 — install & register ainuKey as a Windows TSF input method.

  Works both from a release package (ainukey.dll sits next to this script) and
  from a dev checkout (DLL under target\x86_64-pc-windows-msvc\release\).

  Copies the DLL into "%ProgramFiles%\ainuKey" — a location every app, including
  UWP / Store apps (AppContainer), is allowed to load from — and registers it
  with regsvr32. Do NOT register the DLL from the \\wsl.localhost\ share: UWP
  apps cannot load from it and it is slow.

  Self-elevates (a UAC prompt appears), because registration writes machine COM
  keys and the TSF profile.

  Usage:
    .\install.ps1              # install + register
    .\install.ps1 -Uninstall   # unregister + remove
#>
param([switch]$Uninstall)
$ErrorActionPreference = 'Stop'

# --- self-elevate if not already admin ---
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    $a = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"")
    if ($Uninstall) { $a += '-Uninstall' }
    Start-Process powershell.exe -Verb RunAs -ArgumentList $a
    return
}

$installDir = Join-Path $env:ProgramFiles 'ainuKey'
$dst        = Join-Path $installDir 'ainukey.dll'

if ($Uninstall) {
    if (Test-Path $dst) {
        & regsvr32.exe /u /s $dst
        Remove-Item $installDir -Recurse -Force -ErrorAction SilentlyContinue
        Write-Host "ainuKey unregistered and removed from $installDir"
    } else {
        Write-Host "Nothing to uninstall ($dst not found)."
    }
    Read-Host "Press Enter to close"
    return
}

# Locate the built DLL: release-zip layout first, then dev-checkout layout.
$candidates = @(
    (Join-Path $PSScriptRoot 'ainukey.dll'),
    (Join-Path $PSScriptRoot 'target\x86_64-pc-windows-msvc\release\ainukey.dll')
)
$src = $candidates | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $src) {
    throw "ainukey.dll not found. Looked in:`n  $($candidates -join "`n  ")`nRun .\build.ps1 first (dev), or unzip the release package and run from there."
}

New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Copy-Item $src $dst -Force
Write-Host "Copied DLL -> $dst"
Write-Host "Registering (regsvr32 will show a success/failure dialog)..."
& regsvr32.exe $dst

Write-Host ""
Write-Host "If registration succeeded: open Notepad, switch input method (Win+Space"
Write-Host "or the taskbar language button) to 'ainuKey', and type romaji."
Write-Host "NOTE: v1 registers the profile under the ja-JP langid, so you may need"
Write-Host "Japanese added under Settings > Time & language > Language for it to appear."
Read-Host "Press Enter to close"
