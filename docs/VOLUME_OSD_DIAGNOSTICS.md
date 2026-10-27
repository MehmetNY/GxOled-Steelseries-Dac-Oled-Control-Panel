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
