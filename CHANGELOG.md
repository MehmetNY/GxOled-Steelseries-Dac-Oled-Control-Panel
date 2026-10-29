## GxOled Advanced v0.5.3-beta.2 — beta fix: a way back to the modern interface from the classic window

> **Pre-release.** Stable stays at v0.5.2. This replaces the earlier pre-release `v0.5.3-beta` (kept, not deleted).

- **Fix:** after choosing *Classic* there was no control in the classic window to return to *Modern* — only the tray menu and `--modern-ui` could. The classic window now has a **Modern UI (Beta)** button in its left rail, visible on every page; it switches to the modern window at once and saves the choice. (Found by the owner while trying the way back.)
- The caption of the classic window reads *v0.5.3 Beta 2*; labels derived from the version handle `-beta.N` (`display_version_of` is unit-tested).
- Everything else is as in `v0.5.3-beta`: the modern interface is optional; switch styles from the tray menu (*Settings window style (Beta)*), from *General → Settings window (Beta)* in the modern window, from the new classic-rail button, or with `--classic-ui` / `--modern-ui`; automatic fall-back to Classic if the modern window fails; config key `ui_style`. Details: `docs/MODERN-UI-BETA.md`; third-party licences: `THIRD_PARTY_NOTICES.md`.
- **Verification status (be precise):** compiled and unit-tested on Windows in CI for the commit named in the release notes. **Not verified on a physical Windows PC / base station:** the new classic-rail button's position and look (it is owner-drawn like the Quit button, placed above it), and everything listed as unverified under `v0.5.3-beta` (modern window behaviour, DPI changes, title-bar colouring, resizing, the profile editor's text control, repaint cost).

## GxOled Advanced v0.5.3-beta — modern settings interface (Beta, optional)

> **Pre-release for trying out a new look.** The stable line stays at v0.5.2 (`main`, `stable`, `v0.5.0`–`v0.5.2` are untouched). If you do not like the new interface, switch back in one click — see *Going back* below.

- **New (Beta): modern settings window.** All seven pages (General, Clock, Animations, GIF Studio, Display, Media, About) and the animation-profile editor were redesigned: sidebar navigation, card layout, a live 128 × 64 OLED panel, toggle/radio cards, a brightness slider, light and dark themes, English/Türkçe, per-monitor DPI scaling, resizable window. The window title bar is still the native Windows one (dark/light coloured through DWM on Windows 11).
- **Nothing was taken away.** The classic Win32 settings window is unchanged (only its version caption now reads *v0.5.3 Beta*) and is still compiled into the same EXE. Modern is the default in this beta; choose the style with
  - the tray menu → **Settings window style (Beta)** → *Modern (Beta)* / *Classic*, or
  - **General → Settings window (Beta)** inside the modern window (*Modern* / *Classic* chips), or
  - the command line: `gxoled.exe --classic-ui` (remembers Classic) / `--modern-ui`.
- **Automatic fall-back to Classic** if the modern window cannot be created, panics inside its window procedure, or a previous start did not reach its first painted frame (crash-guard marker `.modern-ui-pending` in `%APPDATA%\GxOled`). The choice is saved, so the next start opens the classic window.
- **How it is built:** new crate `gxoled_ui` (no OS dependency) draws each window as SVG with the pure-Rust `resvg` renderer and embedded fonts/icons; `gxoled_app/src/modern_window.rs` is a small Win32 host (HWND, input, DPI, DWM), `modern_ui.rs` maps the UI to the existing `MenuEvent`s with the same control ids as the classic window, `settings_ui.rs` chooses between the two. No browser, WebView, GPU or extra runtime is needed. Rust remains the only OLED writer.
- **Config:** new optional key `ui_style` (`"Modern"` or `"Classic"`) in `config.toml`; files without it open Modern in this beta. Older GxOled versions ignore the key.
- **Third-party:** Inter fonts (SIL OFL 1.1), Lucide icons (ISC), `resvg`/`usvg`/`tiny-skia` and their dependencies (MIT / Apache-2.0 / BSD) — see `THIRD_PARTY_NOTICES.md` and `gxoled_ui/licenses/`.
- **Verification status (be precise):** compiled and unit-tested on Windows in CI (see the release notes for the commit); the UI toolkit itself is unit-tested (interaction, layout, rendering to PNG in English/Turkish, dark/light). **Not yet verified on a physical Windows PC / base station:** window behaviour, DPI changes, title-bar colouring, resizing, the native text control of the profile editor, and the cost of repainting while hovering. Please report anything that looks wrong — and use Classic meanwhile.
- **Not part of this beta:** a custom title bar, and the GIF editor inspired by GGIF-Studio (later phase).

## GxOled Advanced v0.5.2 — volume screen hand-back fixed, selectable volume-screen time

- **Fix:** after turning the DAC knob GxOled came back 4–6 s late instead of the intended 2.5 s. Measured cause (60 fps video aligned with a frame log, Nova Pro Wireless `12E5`): the base station discards every host frame for about 3.5 s after a dial report, so a frame written when the 2.5 s hold ended stayed invisible. GxOled now sends the documented `0x95` ("return to the SteelSeries UI") immediately before the first frame after the hold; that frame is accepted and the native screen is gone about 2.6 s after the last report. Details and limits: `docs/VOLUME_OSD_HANDOFF.md`.
- **New setting:** *Display → Volume screen time* (settings window) and *Volume screen time* (tray menu): 1, 1.5, 2, 2.5 (default), 3, 4 or 5 s, saved as `volume_osd_hold_ms` in `config.toml`. The duplicate-report filter follows the chosen time.
- Windows: a second GxOled instance is refused with a message instead of two copies fighting over the OLED.
- Windows build helper `Validate-Windows.cmd` builds in a short folder (avoids MSBuild path-length error FTK1011); repository docs reorganised under `docs/`.
- Verified on hardware by the owner with the 2.5 s default (camera test); the other times use the same mechanism but were not individually filmed.

## GxOled Advanced v0.5.1 — DAC volume OSD handoff

- Keep the native DAC volume overlay in control for 2.5 seconds after a genuine dial change. Ignore short-interval repeated HID reports of the same level, which could rearm the timer and extend handoff toward 5–6 seconds.
- Preserve previously fixed clock/media layer cleanup, the existing settings UI geometry, and the embedded SteelClock-Go renderer.
- Add regression tests for duplicate dial reports, changing dial levels, and new dial actions after a quiet interval.
- Update the right-hand Windows settings label, the tray version label, and the Cargo workspace version to v0.5.1.
- Known issue (hardware-measured 2026-10-28, fixed in v0.5.2): the panel still returned to GxOled about 4–6 s after a dial turn, because the base station discards host frames for ~3.5 s; the hold was not the cause.

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
