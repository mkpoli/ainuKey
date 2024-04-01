$dir = "C:\Users\User\Desktop\ainuKey\"
if (!(Test-Path $dir)) {
    New-Item -ItemType directory -Path $dir
}
Remove-Item -Path $dir\ainuKey.dll -ErrorAction SilentlyContinue
Copy-Item -Path $PSScriptRoot\zig-out\lib\ainuKey.dll -Destination $dir -Force

# Print the path of the copied file
Write-Host "Copied ainuKey.dll to $dir ainuKey.dll"
