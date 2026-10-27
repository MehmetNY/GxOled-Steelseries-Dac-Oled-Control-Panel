# DAC volume-OSD timing investigation (no stable release change)

## Why this diagnostic build exists

The original 2026-10-18 code patch (confirmed on hardware) suppressed OLED writes for 2.5 seconds after a volume event. Current v0.5.1 still uses 2.5 seconds but its HID classification and deduplication logic evolved. Physical display handoff has repeatedly been measured as 5–6 seconds. The source alone cannot establish whether the extra time occurs before OLED writes resume, while a HID command is in flight, or inside DAC firmware.

## Procedure

1. Download the Windows MSVC `gxoled.exe` from the GitHub Actions artifact for this diagnostic branch. It is a test EXE, not a new stable release.
2. Exit all existing GxOled/ggoled processes via their tray icons. Do not run the stable EXE at the same time; the app does not yet enforce a singleton.
3. Run PowerShell from the directory containing `Test-VolumeOSD.ps1`:

```powershell
.\Test-VolumeOSD.ps1 -ExePath 'C:\path\to\gxoled.exe' -Mode default
```

4. Turn the physical volume knob once, release it, and measure the delay until GxOled reappears. Then Quit GxOled from its tray menu.
5. Repeat with `-Mode legacy` (the exact 2026-10-18 event-arming semantics) and `-Mode no-hold` (experiment only; the firmware overlay may flicker or be immediately overwritten).
6. Check `%TEMP%\GxOled-OSD-Diagnostics\`. For each trial, note the physical delay and copy the corresponding log. Logs contain local event time, volume level, PID, suppression expiration, USB read timing, and the first three successful HID frame writes. They do **not** establish which pixels the firmware actually displayed.

## Interpreting data

- `HOLD_ARM` to `HOLD_END` ≈2500ms followed promptly by `FRAME_OK`, but physical UI is still visible for 5–6s: likely device-side overlay priority, competing process, or firmware deferring custom images.
- `HOLD_END` occurs at 5–6s: software event timing/re-arm bug. Inspect the chronological `DIAL` reports, accepted flags, and `VOLUME_QUERY` reports.
- `HID_READ_SLOW` or `FRAME_ERROR` appears: investigate HID scheduling or USB errors before modifying rendering.
- `no-hold` returns to GxOled immediately: native OSD can be overridden; software suppression is the cause.
- `no-hold` still waits 5–6s despite successful `FRAME_OK`: firmware display priority is implicated; do not ship arbitrary shorter timeouts as a fix.

A stable v0.5.1 Release must **not** be replaced until at least one tested change actually restores the intended native volume overlay and approximately 2.5-second handoff on physical hardware.

## Findings from the user's October 28 hardware trials

| Trial | End-of-hold / first successful USB OLED write | Physical return |
| --- | --- | --- |
| Default | Last DIAL 16,601ms, HOLD_END 19,101ms, FRAME_OK 19,105ms (2,504ms later) | ~6 seconds |
| Legacy | Last DIAL in the final trial 42,467ms, FRAME_OK 44,971ms (2,504ms later) | 4–5 seconds |
| No-hold | Last DIAL in first trial 8,176ms, FRAME_OK 8,379ms (203ms later) | ~6 seconds |

No-hold is decisive: the program is **successfully sending** custom OLED frames within 203ms, but the actual panel does not visibly relinquish its native volume UI for several seconds. Thus a 2,500ms software constant cannot, by itself, explain the physical delay. A successful HID feature transfer is not proof that the firmware chose to display the frame.

### Next isolated test (important)

Repeat **only `no-hold`** with SteelSeries GG **fully exited using the system-tray Quit option**, and with Sonar audio routing implications understood. The helper now writes a separate `.processes.txt` snapshot. Compare physical return time with GG running versus closed. SteelClock-Go's GameDAC troubleshooting documentation separately warns that SteelSeries GG can periodically redraw or override the base-station OLED while sharing the same device HID interface.

- If handoff becomes immediate when GG is closed: investigate GG/Engine process contention, avoiding automatic termination and preserving Sonar functionality.
- If handoff remains 5–6 seconds even when GG is closed and `no-hold` writes succeed: strong evidence for device-side native-volume-OSD priority. The currently documented USB protocol offers a draw command (`0x93`), return-to-native-UI (`0x95`), and brightness (`0x85`); **none is documented as a safe native-OSD-dismiss command**. Do not send unverified HID commands.
- If multiple GxOled-related processes appear: isolate process conflicts before altering DAC firmware logic. The launch helper already refuses to run when it detects an active GxOled instance.

This remains diagnostic work; leave the v0.5.1 stable Release unchanged until a physically verified solution exists.

## Fourth A/B trial: bounded OLED frame pulse

The user verified that with SteelSeries GG **entirely closed**, `no-hold` still takes 6+ seconds visually. The `pulse` mode therefore isolates a different hypothesis: the Nova firmware might not relinquish priority to 1Hz custom writes while the native volume OSD is active, but may respond to more frequent *valid* writes.

```powershell
.\Test-VolumeOSD.ps1 -ExePath '.\gxoled.exe' -Mode pulse
```

This mode sends valid 0x93 bitmap frames at **at most 10 Hz for up to 7 seconds after the last dial event**, then falls back to the normal 1Hz heartbeat. It sends no undocumented command and changes no DAC configuration or persistent settings. With SteelSeries GG still closed, turn the knob briefly once and measure until GxOled visibly returns. Record whether it returns immediately, after 2-3s, or after ~6s. Some brief screen contention/flicker is possible during this diagnostic test.

Additionally, with GxOled **completely exited** (not just hiding the settings window), time how long the **DAC's own** volume-screen overlay stays visible after turning the dial. That baseline is essential for distinguishing the firmware's native overlay lifetime from application behavior. Do not infer physical display changes from `FRAME_OK`: it is only an acknowledged USB write.

If `pulse` still takes ~6 seconds despite `FRAME_OK` writes at 100ms intervals, reducing the app timer or increasing redraw frequency further will not be a credible solution. Investigate firmware-native display priority, DAC menu options/firmware changes, and baseline native OSD timing instead. Protect all stable releases until hardware-verified.
