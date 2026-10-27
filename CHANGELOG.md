## 2026-10-24 – Two-column settings recovery

- Restored a separate right-side OLED preview and About QR image instead of stacking them above the settings.
- Stabilized DPI scaling independently of window width and kept the navigation Quit action accessible on every page.
- Made animation profile directory notes wrap cleanly and prevented mouse-wheel-only brightness changes.
- Added explicit upstream credit to JerwuQu/ggoled and pozitronik/steelclock-go, with the donation note below the QR.
- Removed an unreachable duplicate localization branch for UI label 716.
- The historical UI notes have been consolidated into the README developer guide.

## Settings layout recovery — 2026-10-24 (experimental)

- Removed the right-hand/hidden OLED preview breakpoint; preview now sits above the page content at every window width, with a single vertical scrolling content column.
- Preserved the non-reentrant Win32 layout/message locking fixes from the prior build.
- Removed broken Clock + Media overlay from the settings window and runtime renderer. Existing configs using that legacy option are migrated to exclusive media playback.
- Fixed Advanced Animation Editor multiline display by converting JSON LF line breaks to native Win32 CRLF; built-in and imported profiles are pretty-printed before editing and export.
- Replaced ten brightness radio buttons with a Windows trackbar (10%–100%, device steps 1–10) and a current-level label.
- Removed redundant second GitHub button. Publisher-provided QR asset is unchanged.
- Original SteelClock Go renderer and Casio drawing are unchanged.
- Windows Cargo and runtime verification must be completed on a Windows machine before release.

## UI Revision — 2026-10-24 (experimental)

- Resizable, DPI-aware native settings pages with vertical scrolling and a small-display initial window size.
- Dark theme default for fresh configurations; new clock labels without changing persisted enum names.
- Unlimited file-backed 128x64 presets for the four original Go animations; import/export and a dark JSON editor.
- Brightness, OLED shift, burn-in, notification, GIF and startup controls added to native settings.
- Bounded clock + media layout, with a 128x24 clipped metadata panel and a full-screen alternative.
- Embedded high-quality publisher QR crop and clickable repository link in About.
- Original Go animation algorithms and bundled Go renderer executable unchanged.
- This version is experimental until Windows Cargo checks, unit tests and device tests pass.

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

## Experimental / SteelClock-Go reset (not a stable release)
- Added embedded upstream-Go animation renderer and original 128x64 profiles (Matrix, Hyperspace, Star Wars, Game of Life), without hand-rewriting their animation logic.
- Added native Win32 Settings dialog and saved UI mode preference (window/tray/hybrid). SDL tray-icon left click still opens its menu; see limitations in README developer guide and go_animations/UPSTREAM.md.
- Recreated 128x64 Casio-style monochrome clock according to supplied layout (week dots, date, 60 seconds ticks). Preserves existing clock idle path.
- Decodes GIFs lazily and disposes unused frame buffers.
- Go tests passed locally; Windows Rust and device verification pending.

### Experimental native-UI follow-up (October 2026)

- Windows-only App opening mode: SDL 3.4 tray message subclass. Tray shows the existing menu; Window opens the settings window on left/right click; Hybrid opens it on left click and keeps the right-click menu. Mode is persisted.
- English default / Turkish UI translation for the principal native and tray controls; 24h/12h digital clock suffixes and Casio weekday abbreviations also follow language.
- Light/Dark native window appearance toggle, with Win32/DWM themed title bar and custom background. OS-native buttons may still follow system accent/style.
- Digital Bold + Large relabelled to Digital Big and enlarged to 1.48x; no new font licenses introduced.
- Casio F-91W: more spacing around four digits and border, distinct LCD-style 1 digit, blinking colon, and fixed canvas position for random clock movement. OLED-level screen shift remains separately configurable.
- Media metadata is now a stand-alone screen when Time is Off, never layered onto clocks/GIF/Go animations.
- Advanced animation profile editor is a local Windows JSON editor. Validated 128x64 overrides are saved to `%APPDATA%/GxOled/profiles/` and used by the embedded unmodified SteelClock-Go widget renderers. Built-in profiles can be restored.
- About page links to https://github.com/MehmetNY/GxOled-v0.4.0. A real donation QR code can be chosen as a PNG; it is deliberately not fabricated because the supplied README contains a placeholder image and wallet `X`.
- `Validate-Windows.ps1` continues to check and test before building. Experimental Windows/Rust code must be compiled and tested on a Windows host before publication.

### Experimental Windows UI compile fix / bundled donation image (October 2026)

- Fixed Windows Rust `E0499` in `settings_window.rs` by creating the sidebar controls before the `make` closure borrows `buttons`.
- Embedded the publisher-provided USDT/TRON donation image directly in the application. The About page displays it at readable size and no longer offers a user-changeable QR image. Previous `%APPDATA%/GxOled/donation-qr.png` files are ignored.
- Repaints the About image immediately on tab selection.
- Added `Validate-Windows.cmd` for a double-click-friendly validation runner that keeps the output visible.
- Rust/Windows compilation and physical scan testing must still be performed on Windows.

### UI scroll & GIF Studio revision — 2026-10-24
- Forced Win32 child-inclusive repaint following native page scrolling and clipped sibling controls to address scroll trails.
- Split GIF playback controls into their own **GIF Studio** settings tab without changing playback engine.
- Added blank **New Profile** editor flow and Ctrl+A selection to multiline JSON editor.
- Added publisher Ko-fi home button and reduced permanent Quit button dimensions.
- See README GIF Studio notes for technical notes and Windows test steps.
