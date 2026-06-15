<#
  build.ps1 — build ainuKey.dll (x64, release, MSVC).

  Plain PowerShell usually works: rustc and embed-resource locate the MSVC
  linker (link.exe) and resource compiler (rc.exe) automatically via vswhere.
  If you hit "link.exe not found" or "rc.exe not found", run this from the
  "x64 Native Tools Command Prompt for VS 2022" instead.

  Prereqs: rustup (https://rustup.rs) and the MSVC build tools + Windows SDK
  (Visual Studio 2022 or "Build Tools for Visual Studio" with the
  "Desktop development with C++" workload).
#>
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

Write-Host "==> rustup target add x86_64-pc-windows-msvc"
rustup target add x86_64-pc-windows-msvc

Write-Host "==> cargo build --release --target x86_64-pc-windows-msvc"
cargo build --release --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

$dll = Join-Path $PSScriptRoot 'target\x86_64-pc-windows-msvc\release\ainukey.dll'
Write-Host ""
Write-Host "Built: $dll"

if (Get-Command dumpbin.exe -ErrorAction SilentlyContinue) {
    Write-Host "Exports (expect all four):"
    & dumpbin.exe /nologo /exports $dll |
        Select-String 'DllGetClassObject|DllCanUnloadNow|DllRegisterServer|DllUnregisterServer'
} else {
    Write-Host "(dumpbin not on PATH — skip export check; the x64 Native Tools prompt provides it)"
}

Write-Host ""
Write-Host "Next:  .\install.ps1   (copies to Program Files + registers; will prompt for UAC)"
