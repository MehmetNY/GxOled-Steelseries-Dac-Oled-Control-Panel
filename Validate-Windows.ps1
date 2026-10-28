# GxOled Windows validation: cargo check -> unit tests -> release build.
# Stops on the first error. Publishes nothing and never touches an installed GxOled/ggoled.
#
# Run it by double-clicking Validate-Windows.cmd (or: powershell -ExecutionPolicy Bypass -File .\Validate-Windows.ps1).
# The finished executable is copied to .\dist\gxoled.exe next to this script.
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

# --- 1. Short build directory ------------------------------------------------------------
# SDL3 is compiled from source through CMake/MSBuild. MSBuild's file tracker cannot write
# files whose full path is longer than 260 characters and fails with
#   FTK1011: could not create the new file tracking log file ... (path not found)
# A repository extracted to Desktop\<name>-main\<name>-main plus target\debug\build\sdl3-sys-...
# easily reaches ~280 characters. So the build output goes to a short folder instead
# (about 160 characters). Set GXOLED_BUILD_DIR to use another short folder.
$buildDir = if ($env:GXOLED_BUILD_DIR) { $env:GXOLED_BUILD_DIR } else { "$env:SystemDrive\gxb" }
$env:CARGO_TARGET_DIR = $buildDir
Write-Host "Build folder (kept short on purpose): $buildDir" -ForegroundColor DarkGray

# --- 2. Tool check with plain instructions -----------------------------------------------
function Require-Tool([string]$Name, [string]$Hint) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "'$Name' was not found. $Hint"
    }
}
Require-Tool 'cargo' 'Install Rust from https://rustup.rs (choose the default MSVC option), then open a NEW window.'
Require-Tool 'cmake' 'Install CMake from https://cmake.org/download and tick "Add CMake to the system PATH".'

function Run-Checked([string]$Title, [scriptblock]$Command) {
    Write-Host "`n==== $Title ====" -ForegroundColor Cyan
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Title failed (exit $LASTEXITCODE). No release executable was produced by this script after this failure."
    }
}

Run-Checked 'Cargo check'   { cargo check --locked -p gxoled_app --features sdl3-static }
Run-Checked 'Unit tests'    { cargo test --locked -p gxoled_lib -p gxoled_draw -p gxoled_app --features gxoled_app/sdl3-static }
Run-Checked 'Release build' { cargo build --locked --release -p gxoled_app --features sdl3-static }

# --- 3. Hand the result back next to this script -----------------------------------------
$built = Join-Path $buildDir 'release\gxoled.exe'
if (-not (Test-Path -LiteralPath $built)) { throw "Build reported success but $built does not exist." }
$dist = Join-Path $PSScriptRoot 'dist'
New-Item -ItemType Directory -Force -Path $dist | Out-Null
Copy-Item -LiteralPath $built -Destination (Join-Path $dist 'gxoled.exe') -Force
$sha = (Get-FileHash -LiteralPath (Join-Path $dist 'gxoled.exe') -Algorithm SHA256).Hash.ToLowerInvariant()

Write-Host "`nValidation complete." -ForegroundColor Green
Write-Host "Executable : $dist\gxoled.exe"
Write-Host "SHA-256    : $sha"
Write-Host 'Note: a successful build proves it compiles; it does not prove it behaves correctly on the DAC.' -ForegroundColor Yellow
