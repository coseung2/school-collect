# C-05 installer lifecycle smoke for the unsigned Windows NSIS build.
#
# Runs on a disposable CI runner: installs the older build silently, checks the
# app and its uninstall entry, upgrades in place with the newer build, checks
# the version moved and nothing was duplicated, uninstalls silently, and checks
# the files and the uninstall entry are gone. Any mismatch fails the run.
#
#   pwsh scripts/installer_lifecycle.ps1 -Older <setup.exe> -Newer <setup.exe> `
#        -OlderVersion 0.2.0 -NewerVersion 0.2.1
#
# The installer is per-user (installMode currentUser), so no elevation is
# needed and everything lands under the current profile.
param(
    [Parameter(Mandatory)] [string] $Older,
    [Parameter(Mandatory)] [string] $Newer,
    [Parameter(Mandatory)] [string] $OlderVersion,
    [Parameter(Mandatory)] [string] $NewerVersion,
    [string] $ProductName = 'School Collect',
    [string] $ExeName = 'school-collect-app.exe'
)
$ErrorActionPreference = 'Stop'

function Fail([string] $message) { throw "installer lifecycle failed: $message" }

# NSIS writes the uninstall entry under HKCU for a per-user install.
function Get-UninstallEntries {
    $roots = @(
        'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall',
        'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall',
        'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall'
    )
    foreach ($root in $roots) {
        if (-not (Test-Path $root)) { continue }
        Get-ChildItem $root | ForEach-Object {
            $entry = Get-ItemProperty $_.PSPath
            if ($entry.DisplayName -eq $ProductName) { $entry }
        }
    }
}

function Invoke-Silently([string] $file, [string[]] $arguments) {
    $process = Start-Process -FilePath $file -ArgumentList $arguments -Wait -PassThru
    if ($process.ExitCode -ne 0) { Fail "$(Split-Path $file -Leaf) exited with $($process.ExitCode)" }
}

function Assert-Installed([string] $expectedVersion) {
    $entries = @(Get-UninstallEntries)
    if ($entries.Count -ne 1) { Fail "expected one uninstall entry, found $($entries.Count)" }
    $entry = $entries[0]
    if ($entry.DisplayVersion -ne $expectedVersion) {
        Fail "uninstall entry reports $($entry.DisplayVersion), expected $expectedVersion"
    }
    # NSIS stores these registry values quoted (`"C:\...\School Collect"`).
    $location = ($entry.InstallLocation -as [string]).Trim().Trim('"')
    if (-not $location) {
        # Some NSIS templates leave InstallLocation empty; derive it from the uninstaller.
        $location = Split-Path (($entry.UninstallString -replace '"', '').Trim())
    }
    $exe = Join-Path $location $ExeName
    if (-not (Test-Path -LiteralPath $exe)) { Fail "app executable missing at $exe" }
    $fileVersion = (Get-Item -LiteralPath $exe).VersionInfo.ProductVersion
    if (-not $fileVersion.StartsWith($expectedVersion)) {
        Fail "installed executable reports $fileVersion, expected $expectedVersion"
    }
    # UninstallString may carry arguments after the quoted path; keep the path.
    $uninstaller = ($entry.UninstallString -as [string]).Trim()
    if ($uninstaller.StartsWith('"')) {
        $uninstaller = $uninstaller.Substring(1, $uninstaller.IndexOf('"', 1) - 1)
    }
    if (-not (Test-Path -LiteralPath $uninstaller)) { Fail "uninstaller missing at $uninstaller" }
    [pscustomobject]@{
        Location    = [System.IO.Path]::GetFullPath($location).TrimEnd('\')
        Exe         = $exe
        Uninstaller = $uninstaller
    }
}

if (@(Get-UninstallEntries).Count -ne 0) { Fail 'the runner already has the app installed' }

"install $OlderVersion"
Invoke-Silently $Older @('/S')
$first = Assert-Installed $OlderVersion
"  installed at $($first.Location)"

"upgrade to $NewerVersion"
Invoke-Silently $Newer @('/S')
$second = Assert-Installed $NewerVersion
if ($second.Location -ne $first.Location) {
    Fail "upgrade moved the install from $($first.Location) to $($second.Location)"
}
"  upgraded in place"

"uninstall"
# `_?=` would keep the uninstaller from copying itself; /S alone is the normal path.
Invoke-Silently $second.Uninstaller @('/S')
# The NSIS uninstaller finishes removing files after its process exits.
for ($i = 0; $i -lt 30 -and (Test-Path -LiteralPath $second.Location); $i++) { Start-Sleep -Seconds 1 }
if (Test-Path -LiteralPath $second.Exe) { Fail "app executable still present after uninstall" }
if (Test-Path -LiteralPath $second.Location) {
    $left = @(Get-ChildItem -LiteralPath $second.Location -Recurse -Force | Select-Object -ExpandProperty FullName)
    Fail "install directory still present after uninstall ($($left.Count) entries: $($left -join ', '))"
}
if (@(Get-UninstallEntries).Count -ne 0) { Fail 'uninstall entry still present after uninstall' }
"  removed the install directory and uninstall entry"

"installer lifecycle OK: install $OlderVersion -> upgrade $NewerVersion -> uninstall"
