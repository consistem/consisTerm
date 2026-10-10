<#
Build and package consisTerm for Windows x64:

  $Dist\consisterm-<version>-windows-x64.exe   the bare executable - what the in-app updater
                                                downloads and swaps in (it takes the first release
                                                asset whose name contains ".exe", so no other
                                                asset may)
  $Dist\consisterm-<version>-windows-x64.zip   the same executable with README, for a manual install

Usage: packaging/windows/package.ps1 [-SkipBuild]

Works with either toolchain: CI builds with MSVC on windows-latest, a local build here uses GNU.
Version: [package] version in Cargo.toml (override: $env:CONSISTERM_VERSION).
#>
param([switch]$SkipBuild)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$Bin = 'consisterm'

function Get-PackageVersion {
    $inPackage = $false
    foreach ($line in Get-Content (Join-Path $Root 'Cargo.toml')) {
        if ($line -match '^\[') { $inPackage = ($line.Trim() -eq '[package]'); continue }
        if ($inPackage -and $line -match '^\s*version\s*=\s*"([^"]+)"') { return $Matches[1] }
    }
    throw "could not read [package] version from Cargo.toml"
}

$Version = if ($env:CONSISTERM_VERSION) { $env:CONSISTERM_VERSION } else { Get-PackageVersion }
$Dist = if ($env:DIST) { $env:DIST } else { Join-Path $Root 'dist\release' }
$TargetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $Root 'target' }
$BaseName = "$Bin-$Version-windows-x64"
New-Item -ItemType Directory -Force $Dist | Out-Null

Write-Host "==> consisTerm $Version for Windows x64"

if (-not $SkipBuild) {
    Push-Location $Root
    try {
        cargo build --release --locked
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
    } finally { Pop-Location }
}

$Exe = Join-Path $TargetDir "release\$Bin.exe"
if (-not (Test-Path $Exe)) { throw "$Exe not found" }

Copy-Item $Exe (Join-Path $Dist "$BaseName.exe") -Force

$Work = Join-Path $TargetDir 'windows-package'
$Stage = Join-Path $Work $BaseName
if (Test-Path $Work) { Remove-Item -Recurse -Force $Work }
New-Item -ItemType Directory -Force $Stage | Out-Null
Copy-Item $Exe (Join-Path $Stage "$Bin.exe")
foreach ($f in 'README.md', 'README.en.md', 'LICENSE') {
    $p = Join-Path $Root $f
    if (Test-Path $p) { Copy-Item $p $Stage }
}
$Zip = Join-Path $Dist "$BaseName.zip"
if (Test-Path $Zip) { Remove-Item -Force $Zip }
Compress-Archive -Path $Stage -DestinationPath $Zip

Write-Host "wrote $(Join-Path $Dist "$BaseName.exe")"
Write-Host "wrote $Zip"
