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

Write-Host "`nValidation complete. Output: $PSScriptRoot\target\release\gxoled.exe" -ForegroundColor Green
