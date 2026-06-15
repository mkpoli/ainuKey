<#
  install.ps1 — install & register ainuKey as the Ainu Windows input method.

  Two halves, because Ainu needs both a machine-wide COM registration and a
  per-user language enable:
    1. Machine-wide (elevated): copy the DLL into Program Files and regsvr32 it
       (registers the COM server + the four transient-LCID input profiles).
    2. Per-user (your normal account, NOT elevated): add Ainu to your language
       list and enable the input method — enable-user.ps1.

  Run this as your normal user; it elevates only the machine half.

  Works from a release package (ainukey.dll + enable-user.ps1 next to this
  script) and from a dev checkout (DLL under target\..., enable-user.ps1 under
  installer\).

  Usage:
    .\install.ps1              # install + register + enable
    .\install.ps1 -Uninstall   # disable + unregister + remove

  NOTE (v0.2): the transient-LCID / InstallLayoutOrTip path is new and needs
  testing on a real Windows machine.
#>
param([switch]$Uninstall, [switch]$MachineStep)
$ErrorActionPreference = 'Stop'

$installDir = Join-Path $env:ProgramFiles 'ainuKey'
$dst        = Join-Path $installDir 'ainukey.dll'

# ---------- Machine half (re-invoked elevated via -MachineStep) ----------
if ($MachineStep) {
    if ($Uninstall) {
        if (Test-Path $dst) {
            & regsvr32.exe /u /s $dst
            Remove-Item $installDir -Recurse -Force -ErrorAction SilentlyContinue
        }
        return
    }
    $src = @(
        (Join-Path $PSScriptRoot 'ainukey.dll'),
        (Join-Path $PSScriptRoot 'target\x86_64-pc-windows-msvc\release\ainukey.dll')
    ) | Where-Object { Test-Path $_ } | Select-Object -First 1
    if (-not $src) { throw "ainukey.dll not found; run .\build.ps1 or unzip the release first." }
    New-Item -ItemType Directory -Force -Path $installDir | Out-Null
    Copy-Item $src $dst -Force
    & regsvr32.exe /s $dst
    if ($LASTEXITCODE -ne 0) { throw "regsvr32 failed ($LASTEXITCODE)" }
    return
}

# ---------- Locate enable-user.ps1 (zip layout or dev checkout) ----------
$enable = @(
    (Join-Path $PSScriptRoot 'enable-user.ps1'),
    (Join-Path $PSScriptRoot 'installer\enable-user.ps1')
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $enable) { throw "enable-user.ps1 not found next to install.ps1." }

function Invoke-MachineStep {
    $a = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$PSCommandPath`"", '-MachineStep')
    if ($Uninstall) { $a += '-Uninstall' }
    $p = Start-Process powershell.exe -Verb RunAs -ArgumentList $a -Wait -PassThru
    if ($p.ExitCode -ne 0) { throw "Elevated machine step failed (exit $($p.ExitCode))." }
}

if ($Uninstall) {
    # Disable per-user FIRST (while the profile still exists), then unregister.
    & $enable -Uninstall
    Write-Host "Unregistering (UAC prompt)..."
    Invoke-MachineStep
    Write-Host "ainuKey uninstalled."
} else {
    # Register machine-wide FIRST, then enable for this user.
    Write-Host "Registering ainuKey (UAC prompt)..."
    Invoke-MachineStep
    & $enable
    Write-Host ""
    Write-Host "ainuKey installed as the Ainu language. Switch input (Win+Space) to Ainu and type romaji."
}
Read-Host "Press Enter to close"
