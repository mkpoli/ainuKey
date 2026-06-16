<#
  install.ps1 — install & register ainuKey (integration build, ja-JP profile).

  Self-elevates (UAC). Copies the DLL into "%ProgramFiles%\ainuKey" and registers
  it. The profile registers under the Japanese langid and is enabled by default,
  so NO per-user step is needed — ainuKey appears in the input switcher under
  Japanese. (Registering it *as Ainu* needs custom-language provisioning that
  Windows doesn't support out of the box; tracked separately.)

  Handles the common "DLL is locked because it's still loaded" case by renaming
  the old DLL before copying the new one.

  Usage:
    .\install.ps1              # install + register
    .\install.ps1 -Uninstall   # unregister + remove
#>
param([switch]$Uninstall)
$ErrorActionPreference = 'Stop'

# --- self-elevate if needed ---
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    $a = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"")
    if ($Uninstall) { $a += '-Uninstall' }
    Start-Process powershell.exe -Verb RunAs -ArgumentList $a
    return
}

$installDir = Join-Path $env:ProgramFiles 'ainuKey'
$dst        = Join-Path $installDir 'ainukey.dll'

# Rename a (possibly loaded/locked) DLL out of the way so it can be replaced.
function Move-LockedAside([string]$path) {
    if (-not (Test-Path $path)) { return }
    # Best-effort unregister of whatever is currently there; a non-zero exit
    # (e.g. it was never registered) is fine — we're about to replace it anyway.
    & regsvr32.exe /u /s $path
    $old = Join-Path (Split-Path $path) ("ainukey-old-{0}.dll" -f (Get-Random))
    try {
        Rename-Item $path $old -Force
    } catch {
        throw "$path is locked (still loaded) and could not be renamed. Sign out/in or reboot once to unload it, then retry."
    }
}

if ($Uninstall) {
    Move-LockedAside $dst
    Remove-Item $installDir -Recurse -Force -ErrorAction SilentlyContinue
    Write-Host "ainuKey unregistered and removed."
    Read-Host "Done. Press Enter to close"
    return
}

# Locate the built DLL (release-zip layout, or dev checkout).
$src = @(
    (Join-Path $PSScriptRoot 'ainukey.dll'),
    (Join-Path $PSScriptRoot 'target\x86_64-pc-windows-msvc\release\ainukey.dll')
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $src) { throw "ainukey.dll not found. Run .\build.ps1 first, or unzip the release." }

New-Item -ItemType Directory -Force -Path $installDir | Out-Null
Move-LockedAside $dst
Copy-Item $src $dst -Force
Write-Host "Installed -> $dst"
Write-Host "Registering..."
& regsvr32.exe /s $dst
if ($LASTEXITCODE -ne 0) {
    throw "regsvr32 failed to register $dst (exit $LASTEXITCODE). The DLL may be a stale/locked copy — sign out/in or reboot once, then retry .\install.ps1."
}
Write-Host "Registered OK."

Write-Host ""
Write-Host "Switch input method (Win+Space) to ainuKey — listed under Japanese —"
Write-Host "and type romaji. (If Japanese isn't in your language list, add it under"
Write-Host "Settings > Time & language > Language.)"
Read-Host "Press Enter to close"
