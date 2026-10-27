# GxOled Windows validation. Stops on the first error. Does not publish anything.
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

function Run-Checked([string]$Title, [scriptblock]$Command) {
    Write-Host "`n==== $Title ====" -ForegroundColor Cyan
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$Title failed (exit $LASTEXITCODE). No release executable was produced by this script after this failure." }
}

Run-Checked 'Cargo check' { cargo check --locked -p gxoled_app --features sdl3-static }
Run-Checked 'Unit tests' { cargo test --locked -p gxoled_draw -p gxoled_app --features gxoled_app/sdl3-static }
Run-Checked 'Release build' { cargo build --locked --release -p gxoled_app --features sdl3-static }

$dist = Join-Path $PSScriptRoot 'release\GxOled-Advanced-v0.5.0'
New-Item -ItemType Directory -Path $dist -Force | Out-Null
Copy-Item (Join-Path $PSScriptRoot 'target\release\gxoled.exe') (Join-Path $dist 'gxoled.exe') -Force
if (Test-Path (Join-Path $PSScriptRoot 'go_animations\gxoled-animations.exe')) {
    Copy-Item (Join-Path $PSScriptRoot 'go_animations\gxoled-animations.exe') (Join-Path $dist 'gxoled-animations.exe') -Force
}
$package = Join-Path $PSScriptRoot 'release\GxOled-Advanced-v0.5.0-Windows-x64.zip'
if (Test-Path $package) { Remove-Item $package -Force }
Compress-Archive -Path (Join-Path $dist '*') -DestinationPath $package -Force
Write-Host "`nValidation complete. EXE: $dist\gxoled.exe" -ForegroundColor Green
Write-Host "Release package: $package" -ForegroundColor Green
