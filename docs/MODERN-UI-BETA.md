# Modern settings interface (v0.5.3 Beta)

The beta replaces the *look* of the settings windows, not the application. Stable v0.5.2 stays as it is; this
interface lives on the branches `release/v0.5.3-beta*` and in the pre-releases `v0.5.3-beta` and `v0.5.3-beta.2` only
(use the newer one: beta.2 adds the classic-window button back to the modern interface). `main` does not contain it
until the owner decides to integrate it.

## Switching between the two interfaces

Any one of these is enough; none needs a re-install.

| How | Effect |
| --- | --- |
| Tray icon → **Settings window style (Beta)** → **Classic** / **Modern (Beta)** | Switches at once and opens the window of that style. Saved. |
| Modern window → **General → Settings window (Beta)** → **Classic** | Same as above, from inside the new window. |
| Classic window → **Modern UI (Beta)** button (left rail, above *Quit*, every page) | Back to the modern window. Added in v0.5.3-beta.2; v0.5.3-beta had no such button. |
| Start `gxoled.exe --classic-ui` / `--modern-ui` | Forces that style and saves it. |
| Edit `%APPDATA%\GxOled\config.toml`: `ui_style = "Classic"` | Same, offline. |
| Install **v0.5.2** | Stable release without any of this code; the extra `ui_style` key is ignored. |

**Türkçe özet:** Yeni arayüzü beğenmezseniz tepsi simgesi → *Ayar penceresi biçimi (Beta)* → *Klasik* seçin (ya da
yeni pencerede *Genel → Ayar penceresi (Beta) → Klasik*, ya da `gxoled.exe --classic-ui`). Klasik pencere aynı EXE'nin
içinde değişmeden durur; en kötü durumda v0.5.2'yi kurmanız yeterlidir.

Automatic fall-back (saved as Classic): the modern window cannot be created · a panic inside its window procedure ·
the previous modern start never painted its first frame (marker file `%APPDATA%\GxOled\.modern-ui-pending`).

## What is where

| Part | Location | OS dependency |
| --- | --- | --- |
| UI toolkit: model, layout, SVG scene, interaction state machine, `resvg` rendering | `gxoled_ui/` | none (runs and is tested on Linux) |
| Win32 host: window class, input, DPI, DWM title-bar colours, native text control of the editor | `gxoled_app/src/modern_window.rs` | Windows (`windows-sys`) |
| Bridge: control ids → `MenuEvent`, profile list/editor logic, OLED mirror, crash guard | `gxoled_app/src/modern_ui.rs` | Windows |
| Style switch and fall-back | `gxoled_app/src/settings_ui.rs` | Windows |
| Classic window | `gxoled_app/src/settings_window.rs` | unchanged except visibility of a few helpers and the caption |

Design rules kept from the project: Rust is the only OLED writer; the classic control ids are reused for the new
UI, so both interfaces drive the same `MenuEvent`s; profile files, config keys and GIF handling are shared.

Rendering: each window is described as SVG (generated in Rust), rasterised by `resvg` with embedded Inter fonts and
inlined Lucide icons, and blitted with `StretchDIBits`. The live 128 × 64 OLED mirror and the QR code are raster
overlays blitted nearest-neighbour. Repaints are event-driven; measured on a Linux build machine a 1450 × 974 frame
takes about 20–24 ms (dark) / 30–36 ms (light). The OLED mirror updates only its own rectangle.

## Verification status

| Level | What |
| --- | --- |
| Unit-tested | `gxoled_ui`: layout, hit testing, keyboard/mouse/scroll/slider/profile-list behaviour, editor surface (26 tests, Linux and Windows CI). `gxoled_app`: control-id → event table, snapshot of the config, volume-time choices (Windows CI). |
| Rendered and reviewed | All pages and the editor, English and Turkish, dark and light, at several sizes/DPIs (PNG review on Linux). |
| Compiled on Windows | Windows MSVC `cargo check`, `cargo test`, `cargo build` in CI for the commit named in the release notes. |
| **Not verified on a physical PC/base station** | Window creation/behaviour on a real desktop, DPI changes between monitors, DWM caption colours (Windows 11), resizing, the native text control, modal file dialogs, repaint cost while hovering on the main thread. |

## Not in this beta

* A custom (frameless) title bar — the native frame is used and coloured through DWM.
* The GIF editor inspired by GGIF-Studio; its licence has to be checked against GPL-3.0 first.

## Third-party material

See [`../THIRD_PARTY_NOTICES.md`](../THIRD_PARTY_NOTICES.md).
