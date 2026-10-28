param(
    [Parameter(Mandatory=$true)][string]$ExePath,
    [ValidateSet("default", "legacy", "no-hold", "pulse")][string]$Mode = "default",
    # Burn a 4-digit frame counter into the bottom-right corner of every sent frame so a
    # camera can show which HID write the panel finally displayed. Works with any -Mode.
    [switch]$Stamp,
    # Automatic experiment cycle (GXOLED_OSD_LAB): every knob turn runs the next experiment from a
    # fixed list; the stamp's leading digit is the experiment number. Implies -Stamp.
    # Some steps send documented device commands (0x95, 0x8d, 0x89) and restore them afterwards.
    [switch]$Lab
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
if ($Lab) { $Stamp = $true }
$modeName = if ($Lab) { "$Mode-lab" } elseif ($Stamp) { "$Mode-stamp" } else { $Mode }
$logPath = Join-Path $dir ("osd-{0}-{1}.log" -f $modeName, (Get-Date -Format 'yyyyMMdd-HHmmss'))
$processLogPath = [System.IO.Path]::ChangeExtension($logPath, ".processes.txt")
$possibleWriters | Select-Object ProcessId, Name, ExecutablePath | Format-List |
    Out-String | Set-Content -LiteralPath $processLogPath -Encoding UTF8
Write-Host "Process snapshot: $processLogPath"
$env:GXOLED_OSD_TRACE_FILE = $logPath
$env:GXOLED_OSD_LEGACY = if ($Mode -eq 'legacy') { '1' } else { '0' }
$env:GXOLED_OSD_HOLD_MS = if ($Mode -in @('no-hold','pulse')) { '0' } else { '2500' }
$env:GXOLED_OSD_PULSE = if ($Mode -eq 'pulse') { '1' } else { '0' }
$env:GXOLED_OSD_STAMP = if ($Stamp) { '1' } else { '0' }
$env:GXOLED_OSD_LAB = if ($Lab) { '1' } else { '0' }

Write-Host "Diagnostic mode: $Mode (frame stamp: $([bool]$Stamp), lab: $([bool]$Lab))"
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
Remove-Item Env:\GXOLED_OSD_PULSE -ErrorAction SilentlyContinue
Remove-Item Env:\GXOLED_OSD_STAMP -ErrorAction SilentlyContinue
Remove-Item Env:\GXOLED_OSD_LAB -ErrorAction SilentlyContinue
