Stop-Process -Name Notepad -ErrorAction SilentlyContinue;
Write-Host "Terminated Notepad";

regsvr32 /u "C:\Users\User\Desktop\ainuKey\ainuKey.dll" /s
Write-Host "Unregistered ainuKey.dll";

$dir = "C:\Users\User\Desktop\ainuKey"
if (!(Test-Path $dir)) {
    New-Item -ItemType directory -Path $dir
    Write-Host "Created directory $dir"
}

& "C:\Program Files (x86)\IObit\IObit Unlocker\IObitUnlocker.exe" /Delete /Advanced "$dir\ainuKey.dll"
TimeOut 1
taskkill /IM IObitUnlocker.exe
Write-Host "Deleted ainuKey.dll from $dir\ainuKey.dll"

TimeOut 1
Copy-Item -Path $PSScriptRoot\zig-out\lib\ainuKey.dll -Destination $dir -Force
Write-Host "Copied ainuKey.dll to $dir\ainuKey.dll"

regsvr32 "C:\Users\User\Desktop\ainuKey\ainuKey.dll" /s
Write-Host "Registered ainuKey.dll";

TimeOut 2
Start-Process "C:\Windows\System32\notepad.exe"
Write-Host "Started Notepad";
