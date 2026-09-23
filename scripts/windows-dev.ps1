param(
    [ValidateSet('check', 'build', 'dev', 'test')]
    [string]$Action = 'check'
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
$rustup = Join-Path $cargoBin 'rustup.exe'
if (-not (Test-Path -LiteralPath $rustup)) {
    throw 'Install Rust through rustup with the Windows MSVC host first.'
}
$env:Path = "$cargoBin;$env:Path"
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path -LiteralPath $vswhere)) {
    throw 'Install Visual Studio Build Tools with Desktop development with C++ first.'
}
$vsRoot = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vsRoot) { throw 'Visual Studio C++ build tools were not found.' }
& (Join-Path $vsRoot 'Common7\Tools\Launch-VsDevShell.ps1') -Arch amd64 -HostArch amd64 -SkipAutomaticLocation

Push-Location $repoRoot
try {
    $compiler = & rustc -vV
    if ($LASTEXITCODE -ne 0 -or -not ($compiler -match 'host: x86_64-pc-windows-msvc')) {
        throw 'The repository requires the pinned x86_64-pc-windows-msvc toolchain on Windows.'
    }
    switch ($Action) {
        'check' { & cargo check --workspace --locked }
        'build' { & pnpm --filter '@school-collect/app' tauri build --no-bundle -- --locked }
        'dev' { & pnpm --filter '@school-collect/app' tauri dev -- --locked }
        'test' { & cargo test --workspace --locked }
    }
    if ($LASTEXITCODE -ne 0) { throw "$Action failed with exit code $LASTEXITCODE" }
} finally {
    Pop-Location
}
