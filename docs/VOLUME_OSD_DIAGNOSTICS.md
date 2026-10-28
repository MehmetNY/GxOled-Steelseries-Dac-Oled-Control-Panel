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

## Exact historical source differential test

The complete `ggoled-volume-osd-bugfix.zip` generated on October 18, 2026 (the version physically verified on October 19 to show roughly 2.5s handoff) is preserved byte-for-byte at `diagnostics/legacy-2026-10-18/ggoled-volume-osd-bugfix.zip`. Its independent CI builds `ggoled-2026-10-18-known-good-comparison.exe` as a **diagnostic artifact only**.

Compare that historical binary against current v0.5.1 with only one process running, the same USB DAC, similar clock/GIF state, and SteelSeries GG closed. The October 18 version uses original application names/config folders, so avoid accepting its startup-on-boot option during this one-time test. It has the old UI and is **not** a stable v0.5.1 replacement. The only purpose of the test is to establish whether the old *known-good executable* still restores the OLED in ~2.5s on the same present hardware.

If it does, prioritize differential debugging of transport, event dispatch, rendering and frame scheduling between original and modern sources before claiming firmware cause. If it does not, the hardware or external environment/firmware behavior has changed since October 19 (or there is a baseline observation mismatch); inspect current DAC native OSD duration. Pulse testing already produced tearing and no speed improvement: do not turn it on by default or ship it.

## Fifth trial: frame stamp (was the early frame buffered, or discarded?)

The user cannot measure a "native-only" baseline: without GxOled the stock UI is always on screen, so there is no overlay that ever closes. The stamp trial replaces that baseline with a measurement that works with the hardware as it is.

```powershell
.\Test-VolumeOSD.ps1 -ExePath '.\gxoled.exe' -Mode no-hold -Stamp
.\Test-VolumeOSD.ps1 -ExePath '.\gxoled.exe' -Mode default -Stamp
```

`-Stamp` (`GXOLED_OSD_STAMP=1`) burns a four-digit frame counter into the bottom-right corner of every frame that is actually sent. The counter is drawn on a copy, so the send cadence (on change, plus the 1 Hz heartbeat) is exactly the normal one. The log gets one `FRAME seq=N since_dial_ms=…` line per sent frame for ten seconds after the last dial report, and each `DIAL` line records `last_seq`.

How to film it: phone at 60 fps, OLED and the volume knob in frame, one short knob turn per take, at least five takes per mode. For each take write down the first counter value that appears after the stock UI disappears, and the time from the knob stopping to that moment.

Look up that counter in the log (`since_dial_ms` of that `seq`) and compare with the filmed return time:

| First visible `seq` was sent… | Meaning |
| --- | --- |
| about when the stock UI disappeared (a late frame) | Frames sent while the stock UI was up are **discarded**; only a write after the firmware lets go is shown. Nothing the host sends earlier can help. |
| right after the last dial report (an early frame, e.g. ~200 ms in `no-hold`) | The firmware **latched** the early frame and showed it late. The display content was ready all along; the delay is the firmware's own timer. |
| in between, or it varies a lot between takes | Mixed behaviour; the spread itself is the finding. Note whether media/GIF was running (see the cadence note below). |

Cadence note: with the clock the app sends about one frame per second. With a media title wider than 128 px (a `Scroll` layer) or a GIF/Go animation it sends up to 30 frames per second. Record which one was active on every take.

## Facts verified in the source (2026-10-28 review)

- After the 2.5 s hold, `force_redraw` is always true (the last frame is more than 1 s old), so the first post-hold frame goes out immediately. Matches the logs (`HOLD_END` → `FRAME_OK` in ~4 ms).
- The HID transport (`create_report`, `draw`, `retry_report`, `get_events`) is byte-for-byte the same in the preserved 2026-10-18 source and in v0.5.1. Only event classification and bounds checks differ. v0.5.1's de-duplication can shorten the hold but never lengthen it.
- Upstream JerwuQu/ggoled has no volume-OSD handling at all; the hold logic is this project's own addition.
- The similar project Nexus-Nexx/arctis-nova-pro-oled uses the same `0x93` two-chunk transport, streams at 30 fps and draws its own volume overlay for 1.5 s after a dial report. Whether the native overlay yields to a 30 fps stream is untested on this DAC and is *not* proposed as a fix here.

## How the fifth trial is run (2026-10-28)

The frame-stamp measurement is distributed as a ready-made Windows package so nobody has to build anything:
CI publishes a **pre-release** (tag `diag-osd-stamp-<sha7>`, never a `v*` tag) containing `gxoled.exe`, the
launchers and a Turkish step-by-step guide. See [`TR-TEST-REHBERI.md`](TR-TEST-REHBERI.md) and
[`../scripts/diagnostics-package/BASLA-BURADAN.txt`](../scripts/diagnostics-package/BASLA-BURADAN.txt).
`BUILD-INFO.txt` inside the ZIP names the source commit and EXE SHA-256.

## Experiments that stay off the table

- **A fixed higher frame rate during the handoff.** The 10 Hz pulse already produced tearing and did not shorten the delay
  (the fifth-trial video analysis below explains why: the frames are discarded).
- **Shortening the timers blindly.** The delay is not the software hold (the hold ends at ~2.5 s and the frame follows in ~4 ms).

Device commands that were earlier excluded as "unverified" (`0x95`, `0x8d`, `0x89`) are **now tried on purpose** in the OSD lab,
because the owner lifted that restriction on 2026-10-28 ("sorunu çözmek ve tüm işlevleri çalışan bir uygulama yapmak yegane
önceliğimiz"). They stay opt-in (`GXOLED_OSD_LAB=1`) until one of them is shown on hardware to shorten the delay.

## Fifth trial result (2026-10-28, hardware, 1080p 60 fps video + stamped logs)

Method: `scripts/analysis/analyze_osd_video.py` finds the OLED in the video, marks every episode in which
the base station's own home screen (top-centre "USB" tab) is visible, aligns video and log time, and names the
first frame (by stamp counter) that was written at or after the moment the native screen disappeared.
The tool's output matches the counters read by eye from the same frames.

| Take | Mode | Last dial report → last frame that was **not** shown | → first frame that **was** shown |
| --- | --- | --- | --- |
| 1 | default (2.5 s hold) | seq 8 @ 4.67 s | seq 9 @ 5.67 s |
| 2 | default | seq 14 @ 3.87 s | seq 15 @ 4.87 s |
| 3 | default | seq 20 @ 5.24 s | seq 21 @ 6.24 s |
| 4 | no-hold | seq 9 @ 4.94 s | seq 10 @ 5.94 s |
| 5 | no-hold | seq 19 @ 4.50 s | seq 20 @ 5.50 s |
| 6 | no-hold | seq 28 @ 5.50 s | seq 29 @ 6.50 s |
| 7 | no-hold | seq 38 @ 4.67 s | seq 39 @ 5.67 s |

What it shows:

1. **Frames written while the native screen is up are discarded, not latched.** Every counter between the dial and
   the ending frame is missing from the video. (Resolves the "latched vs discarded" question of the fifth trial.)
2. **The native screen disappears at the instant a host frame is written after the firmware's lock expires.**
   In the three default takes the video shows the native screen ending 2.157–2.160 s (video clock) after the
   log time of the ending frame, i.e. a constant lag, not a timer of its own.
3. The lock ends 3.9–6.5 s after the last dial report; the exact value is only known to within the 1 s heartbeat
   (last discarded frame, first shown frame). It is **not** a constant offset from the last report (take 2 ends
   before take 3's last discarded frame) and is roughly 6.1 s after the *first* report in most takes; continued
   turning can extend it (take 5). The firmware's rule is not fully known.
4. No host-side timing (hold 2.5 s, no hold, 10 Hz pulse measured earlier) changes this, because the frames are
   ignored. A faster heartbeat after the lock could shave at most the wait for the next frame (≤ 1 s).

Consequence: the remaining levers are **device commands**. The October 18 build was physically reported as
"about 2.5 s" and behaves like v0.5.1 today, so the device state differs from then. The OSD lab (`GXOLED_OSD_LAB=1`,
`scripts/diagnostics-package/`) tries, one per knob turn: `0x95` return-to-UI at three moments, the `0x8d`
"Sonar present" flag (SteelSeries GG sets it while running; the base station keeps it until told otherwise or
power-cycled) and the `0x89` home-screen type. It also logs the device's own UX settings (`0x80`) and every
unrecognised inbound report, which may reveal a signal for "native screen ended" and make future tests camera-free.
