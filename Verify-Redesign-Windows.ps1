# Run from the extracted source root.  Keeps the original Windows validator intact.
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
& "$PSScriptRoot\Validate-Windows.ps1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
Write-Host "UI manual checks: sections order, preview, dark/light, JSON editor min size, animation playback." -ForegroundColor Cyan
