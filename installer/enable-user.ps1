<#
  enable-user.ps1 — the PER-USER half of installing ainuKey.

  Ainu (BCP-47 "ain") has no Windows LCID, so it uses a *transient* LCID. The
  DLL's DllRegisterServer (machine-wide, elevated) registers the profile against
  all four transient slots; THIS script, which must run as the actual (non-
  elevated) user, adds "ain" to the user's language list — which makes Windows
  assign it a transient LCID — and enables the input method via InstallLayoutOrTip.

  Must run in the target user's context (NOT elevated), or it modifies the wrong
  user's language list.

  Usage:  enable-user.ps1            # add "ain" + enable ainuKey
          enable-user.ps1 -Uninstall # disable ainuKey (+ optionally remove "ain")

  NOTE (v0.2): this transient-LCID + InstallLayoutOrTip path is new and needs
  verification on real Windows. References: [MS-LCID] "Locale Names without
  LCIDs"; Keyman GetLayoutInstallString / Set-WinUserLanguageList.
#>
param([switch]$Uninstall, [switch]$RemoveLanguage)
$ErrorActionPreference = 'Stop'

# ainuKey identity (must match src/guids.rs).
$CLSID      = '{5ECECCEB-271D-4675-8EE5-8D129EF0CA08}'
$ProfileGui = '{5ECECCEC-271D-4675-8EE5-8D129EF0CA08}'
$LangTag    = 'ain'

# input.dll (InstallLayoutOrTip) has no import lib — bind by name. kernel32 for
# LocaleNameToLCID to read the transient LCID Windows assigned to "ain".
$Api = Add-Type -Namespace AinuKey -Name Native -PassThru -MemberDefinition @"
[System.Runtime.InteropServices.DllImport("input.dll", CharSet=System.Runtime.InteropServices.CharSet.Unicode, SetLastError=true)]
public static extern bool InstallLayoutOrTip(string psz, uint dwFlags);
[System.Runtime.InteropServices.DllImport("kernel32.dll", CharSet=System.Runtime.InteropServices.CharSet.Unicode)]
public static extern int LocaleNameToLCID(string lpName, uint dwFlags);
"@

$ILOT_UNINSTALL = 0x1

function Get-AinuLcid {
    $lcid = $Api::LocaleNameToLCID($LangTag, 0)
    # > 0x1000 means a real/transient LCID; 0x1000 == LOCALE_CUSTOM_UNSPECIFIED.
    if ($lcid -le 0x1000) { return $null }
    return $lcid
}

function Get-InstallString([int]$lcid) {
    # Keyman format: '%04.4x:{CLSID}{ProfileGUID}'  (4 hex digits, no 0x prefix).
    return ('{0:x4}:{1}{2}' -f $lcid, $CLSID, $ProfileGui)
}

if ($Uninstall) {
    $lcid = Get-AinuLcid
    if ($lcid) {
        $s = Get-InstallString $lcid
        Write-Host "Disabling ainuKey: $s"
        [void]$Api::InstallLayoutOrTip($s, $ILOT_UNINSTALL)
    }
    if ($RemoveLanguage) {
        $list = Get-WinUserLanguageList
        $keep = $list | Where-Object { $_.LanguageTag -ne $LangTag }
        if ($keep.Count -ne $list.Count) { Set-WinUserLanguageList $keep -Force }
    }
    return
}

# Install: ensure "ain" is in the user's language list (this is what triggers the
# transient-LCID assignment), then enable the TIP for that LCID.
$list = Get-WinUserLanguageList
if (-not ($list | Where-Object { $_.LanguageTag -eq $LangTag })) {
    Write-Host "Adding Ainu ('$LangTag') to your language list..."
    $list.Add($LangTag)
    Set-WinUserLanguageList $list -Force
}

$lcid = Get-AinuLcid
if (-not $lcid) {
    throw "Windows did not assign a transient LCID to '$LangTag'. All four transient slots (0x2000/0x2400/0x2800/0x2C00) may be in use by other input methods; free one and retry."
}
$s = Get-InstallString $lcid
Write-Host ("Enabling ainuKey under transient LCID 0x{0:x4}: {1}" -f $lcid, $s)
[void]$Api::InstallLayoutOrTip($s, 0)
Write-Host "Done. Switch input (Win+Space) to Ainu and type romaji."
