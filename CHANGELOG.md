# GxOled Changelog

# Changelog

## 0.4.0

- Renamed the application, desktop executable, CLI, internal crate directories, tray/menu labels, icons, startup registration, and packaging metadata to GxOled. Existing GgOled settings and Windows startup registration are migrated on first launch.

- Added selectable digital and analog clock styles.
- Added GIF selection and Start / Stop controls in the system-tray menu, including persistence of the selected GIF and playback state.
- Added a Screen brightness submenu for device levels 1–10.
- Added Random Movements and No overlapping Pixels clock burn-in modes.
- Added a Windows-only startup toggle.
- Kept the base station's native volume OSD in control during physical dial changes, without treating routine volume-status replies as OSD activity; animation timing is preserved while OLED writes are paused.
- Added a regression test distinguishing physical volume-dial reports from routine volume-query replies.
- Made settings writes atomic and bounded GIF decoding in the desktop-app loader.
- Corrected GIF frame-delay scheduling so the first frame is shown for its configured duration and delayed rendering does not trigger a rapid catch-up burst.
- Reapply the saved brightness after the base station reconnects, and safely handle short or malformed HID report descriptors.
- Invalid saved brightness levels are ignored, and missing/invalid custom fonts fall back to the built-in font instead of panicking.
- Kept existing configuration files compatible by defaulting newly added fields.
- Added unit-test and native Windows MSVC build gates before publishing tagged releases.
- Bound custom-font file size and sanitized/truncated external media metadata before rendering.
