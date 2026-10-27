param(
    [Parameter(Mandatory=$true)][string]$ExePath,
    [ValidateSet("default", "legacy", "no-hold")][string]$Mode = "default"
)

$ErrorActionPreference = "Stop"
$exe = (Resolve-Path -LiteralPath $ExePath).Path
$existing = @(Get-CimInstance Win32_Process | Where-Object {
    $_.Name -match '(?i)^(gxoled|ggoled)(\.advanced[^\\]*)?\.exe$' -or
    $_.ExecutablePath -match '(?i)\\(gxoled|ggoled)[^\\]*\.exe$'
})
if ($existing.Count -gt 0) {
    $existing | Select-Object ProcessId, Name, ExecutablePath | Format-Table -AutoSize
    throw "Another GxOled/ggoled instance is running. Quit every GxOled instance via its tray icon before diagnosing. No processes were stopped by this script."
}

# Only inspect other software that could compete for DAC OLED ownership.
# Never close or kill it automatically: SteelSeries Sonar may carry active audio.
$possibleWriters = @(Get-CimInstance Win32_Process | Where-Object {
    $_.Name -match '(?i)(steelseries|sonar|steelclock|ggoled|gxoled)' -or
    $_.ExecutablePath -match '(?i)(steelseries|steelclock)'
})
if ($possibleWriters.Count -gt 0) {
    Write-Warning "Possible competing device processes found. This is NOT proof of interference:"
    $possibleWriters | Select-Object ProcessId, Name, ExecutablePath | Format-Table -AutoSize
    Write-Host "For the isolation trial, close SteelSeries GG completely via its tray icon (if appropriate)."
    Write-Host "Sonar-dependent audio may be interrupted; no processes have been stopped by this script."
}

$dir = Join-Path $env:TEMP 'GxOled-OSD-Diagnostics'
New-Item -ItemType Directory -Path $dir -Force | Out-Null
$logPath = Join-Path $dir ("osd-{0}-{1}.log" -f $Mode, (Get-Date -Format 'yyyyMMdd-HHmmss'))
$processLogPath = [System.IO.Path]::ChangeExtension($logPath, ".processes.txt")
$possibleWriters | Select-Object ProcessId, Name, ExecutablePath | Format-List |
    Out-String | Set-Content -LiteralPath $processLogPath -Encoding UTF8
Write-Host "Process snapshot: $processLogPath"
$env:GXOLED_OSD_TRACE_FILE = $logPath
$env:GXOLED_OSD_LEGACY = if ($Mode -eq 'legacy') { '1' } else { '0' }
$env:GXOLED_OSD_HOLD_MS = if ($Mode -eq 'no-hold') { '0' } else { '2500' }

Write-Host "Diagnostic mode: $Mode"
Write-Host "Log file: $logPath"
Write-Host "Start GxOled, rotate the DAC volume once, then stop moving it."
Write-Host "Observe the physical OLED return delay. Exit via 'Quit GxOled' in the tray."
$p = Start-Process -FilePath $exe -PassThru
$p.WaitForExit()
Write-Host "Diagnostic complete. Log: $logPath"
if (Test-Path $logPath) { Get-Content -LiteralPath $logPath | Select-Object -Last 50 }
Remove-Item Env:\GXOLED_OSD_TRACE_FILE -ErrorAction SilentlyContinue
Remove-Item Env:\GXOLED_OSD_LEGACY -ErrorAction SilentlyContinue
Remove-Item Env:\GXOLED_OSD_HOLD_MS -ErrorAction SilentlyContinue
