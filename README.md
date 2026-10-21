# GxOled — SteelSeries Base Station OLED Enhancements (v0.4.0)

**A community-maintained desktop-app enhancement of [JerwuQu/ggoled](https://github.com/JerwuQu/ggoled).** GxOled is a separately maintained application derived from the upstream source; its maintainer does not own or represent the upstream repository. It is an unofficial derivative, not an official upstream release, and it is not affiliated with, endorsed by, or sponsored by SteelSeries.

The project keeps the upstream USB HID implementation and command-line tool as its foundation, while adding desktop-app features for the 128×64 OLED on compatible SteelSeries base stations.

> **Please preserve upstream attribution.** The original project and its contributors made the core device integration possible. This derivative retains the upstream GPL license and source attribution. See [`LICENSE`](LICENSE).

## Some pictures: 

<table align="center">
  <!-- First Row -->
  <tr>
    <td align="center" width="33%">
      <img src="https://github.com/user-attachments/assets/08087e53-ac59-4d96-aae2-86bff951c585" alt="Ekran Görüntüsü 1" style="max-height: 200px; width: auto;"/><br />
      <sub><b>SS 1</b></sub>
    </td>
    <td align="center" width="33%">
      <img src="https://github.com/user-attachments/assets/fb2c1cee-2b0d-46e3-b5f1-37fb7fe9b70e" alt="Ekran Görüntüsü 2" style="max-height: 200px; width: auto;"/><br />
      <sub><b>SS 2</b></sub>
    </td>
    <td align="center" width="33%">
      <img src="https://github.com/user-attachments/assets/e9859914-d039-4977-921e-a2932d05d144" alt="Ekran Görüntüsü 3" style="max-height: 200px; width: auto;"/><br />
      <sub><b>SS 3</b></sub>
    </td>
  </tr>
  <!-- Second Row -->
  <tr>
    <td align="center" width="33%">
      <img src="https://github.com/user-attachments/assets/6509c9bb-7f34-4ff1-be66-3aa4269433f9" alt="Ekran Görüntüsü 4" style="max-height: 200px; width: auto;"/><br />
      <sub><b>SS 4</b></sub>
    </td>
    <td align="center" width="33%">
      <img src="https://github.com/user-attachments/assets/00c89698-fb6e-40c8-aa4c-09924e6991c4" alt="Ekran Görüntüsü 5" style="max-height: 200px; width: auto;"/><br />
      <sub><b>SS 5</b></sub>
    </td>
    <td align="center" width="33%">
      <img src="https://github.com/user-attachments/assets/3a29fcb3-6db1-45e6-bc81-ceef59a82684" alt="Ekran Görüntüsü 6" style="max-height: 200px; width: auto;"/><br />
      <sub><b>SS 6</b></sub>
    </td>
  </tr>
</table>


## Features added in this derivative

The GxOled system-tray application includes the following enhancements over the upstream desktop app:

- **Four clock styles:** Digital → Original, Digital → Bold + Large, Analog → Round, and Analog → Square. Both analog styles have hour, minute, and second hands, numerals at 12/3/6/9, and tick marks at the other positions.
- **Clock OLED burn-in mitigation:** `Random Movements` and `No overlapping Pixels` reposition the clock at minute boundaries. These features spread pixel use; they cannot guarantee that OLED aging or burn-in will never happen.
- **GIF playback UI:** `Animation → Add GIF` opens the native file picker. A separate `Start / Stop animation` checkbox controls playback. The last GIF path and playback state are saved between launches.
- **Screen brightness control:** a tray submenu exposes the device's protocol levels 1–10, labelled 10%–100%. These labels map to protocol steps; they do not claim a factory-measured linear luminance curve. Brightness is not changed until the user selects a level, and the saved level is re-applied after the base station reconnects.
- **Windows startup toggle:** `Start GxOled with Windows` adds or removes the current executable from the current user's Windows `Run` registry key. This menu item is Windows-only.
- **Native volume OSD compatibility:** physical base-station volume-dial events are distinguished from routine volume-status replies. OLED writes pause while the base station shows its own volume overlay, preventing GxOled from competing with the native screen; animation resumes without a rapid catch-up burst.
- **Live clock rendering:** the clock continues to update while the Windows tray menu is open, rather than freezing on an old rendered time.
- **Persistent configuration:** settings are saved atomically to the user's configuration file. On Windows this is normally `%APPDATA%\GxOled\config.toml`. Existing `%APPDATA%\ggoled_app.toml` settings are migrated on first launch; the original file is left as a backup. The old Windows startup entry is also migrated to the GxOled entry.
- **Input hardening:** the desktop GIF loader bounds file size, canvas dimensions, frame count, and aggregate decoded pixels; custom font files are size-limited; external media text is sanitized and length-limited; invalid saved brightness and unavailable custom fonts are handled safely.

The command-line interface (`gxoled-cli`) remains available for scripting and direct display operations. The existing burn-in-related screen shifter and idle screensaver options are retained.

## Supported devices and compatibility scope

The program communicates with the **USB base station**, not directly with the headset. Compatibility therefore depends on the base station's USB product ID, HID interfaces, and report protocol. Headset variants connected through one of the supported base stations use that same base-station integration.

| Base-station / product configuration | USB product ID | Status in this source |
|---|---:|---|
| SteelSeries Arctis Nova Pro Wired | `0x12CB` | Supported by the device matcher |
| SteelSeries Arctis Nova Pro Wired (Xbox) | `0x12CD` | Supported by the device matcher |
| SteelSeries Arctis Nova Pro Wireless | `0x12E0` | Supported by the device matcher |
| SteelSeries Arctis Nova Pro Wireless (Xbox) | `0x12E5` | Supported by the device matcher |
| SteelSeries Arctis Nova Pro Wireless (Xbox White) | `0x225D` | Supported by the device matcher |
| SteelSeries Arctis Nova Elite base station | `0x2244` | Experimental / not hardware-verified by this project maintainer |

**This is not a claim of compatibility with every SteelSeries headset or every SteelSeries base station.** The current code explicitly matches the IDs listed above. Devices such as Arctis Pro Wireless/Wired and Nova Pro Omni are not confirmed by this source's device matcher; do not assume compatibility solely because the headset carries the SteelSeries name. Reports and patches for additional hardware are welcome.

## Download and install

### Windows

1. Download `gxoled.exe` from this repository's **[Releases](https://github.com/MehmetNY/GxOled-v0.4.0/releases/tag/stable)** page.
2. Run it. The desktop application starts in the system tray; right-click the tray icon to configure it.
3. The CLI `gxoled-cli.exe` may also be attached to a release for users who want command-line controls.

For building the desktop application locally, see [Building from source](#building-from-source). On Windows/MSVC, extract the source to a short path such as `C:\gx`; SDL3's CMake build can fail when nested temporary paths become too long.

### Linux

Use a Flatpak asset attached to this repository's release if one is provided, or build from source. The Flatpak bundle and helper files are under [`flatpak/`](flatpak/). For direct USB access outside Flatpak, install [`11-steelseries-arctis-nova.rules`](11-steelseries-arctis-nova.rules) into `/etc/udev/rules.d/` and reload udev rules:

```sh
sudo cp 11-steelseries-arctis-nova.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules && sudo udevadm trigger
```

The Flatpak build helper downloads Flathub's `shared-modules` repository into an ignored working directory when the required manifest is not already available. The external module is not vendored into this project's source tree. The Flatpak ID currently uses `io.github.gxoled.GxOled`; if your GitHub repository owner is not `gxoled`, change this ID and its filenames before submitting the app to Flathub.

### macOS

The source includes a macOS platform adapter, but a prebuilt macOS release is not currently promised. Build and test for your environment before relying on it.

## Building from source

Install the Rust toolchain and the native build tools required by your platform. Run all commands from the directory containing the top-level `Cargo.toml`.

Check the desktop app:

```powershell
cargo check --locked -p gxoled_app
```

Run the regression tests used for the desktop app and device/drawing code:

```powershell
cargo test --locked -p gxoled_app -p gxoled_lib -p gxoled_draw --features gxoled_app/sdl3-static
```

Build the Windows desktop app with SDL3 compiled from source:

```powershell
cargo build --locked --release -p gxoled_app --features sdl3-static
```

Build the command-line tool separately:

```powershell
cargo build --locked --release -p gxoled_cli
```

Expected outputs are `target/release/gxoled.exe` and `target/release/gxoled-cli.exe` on Windows, or their extensionless equivalents on Unix-like systems. The source repository does not include `target/` or prebuilt binaries; compiled artifacts should be attached to GitHub Releases rather than committed into the source tree.

### systemd user service (Linux)

```sh
mkdir -p ~/.config/systemd/user/
cp gxoled.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now gxoled.service
```

### Custom font

Bitmap fonts are recommended for the small OLED to avoid rasterization artifacts, though TTF/OTF fonts can also be used. Edit `%APPDATA%\GxOled\config.toml` on Windows, or `$XDG_CONFIG_HOME/GxOled/config.toml` on Linux, and set:

```toml
[font]
path = 'C:\Path\To\Font.ttf'
size = 16.0
```

Restart the app after changing the font configuration. A missing, invalid, or over-limit custom font falls back to the built-in font.

## Security and dependency check

Optional security/dependency check:

```powershell
cargo install cargo-audit
cargo audit
```

`cargo audit` may report maintenance or yanked-crate warnings even when no vulnerability advisory is found. Review the current output rather than treating every warning as an exploitable vulnerability.

## CLI usage examples

Run `gxoled-cli --help` for the full command list.

```sh
gxoled-cli brightness 1
gxoled-cli text "Hello, World!"
gxoled-cli img cool_image.png
gxoled-cli anim -r 10 -l 20 frame1.png frame2.png frame3.png
gxoled-cli anim animation.gif
```

For a video, extract frames with FFmpeg first:

```sh
ffmpeg -i YOURVIDEO.mp4 -r 20 -vf "scale=w=128:h=64:force_original_aspect_ratio=1" frames/%05d.png
# Bash
gxoled-cli anim -r 20 frames/*
# PowerShell
gxoled-cli anim -r 20 (Get-ChildItem frames | ForEach-Object { $_.FullName })
```

## Roadmap (ideas, not implemented in v0.4.0)

- **GIF/image framing studio:** a visual selection and preview screen with finer control over aspect ratio, fit/crop (`contain`, `cover`, `stretch`), zoom, positioning, rotation, flipping, image tone, and threshold/dithering. The design will take inspiration from [D4rkTT/GGIF-Studio](https://github.com/D4rkTT/GGIF-Studio). This is a reference for ideas, not a claim of affiliation or copied code.
- **Additional OLED widgets:** consider implementing selected, suitable widget ideas similar in spirit to the clock, gauges, and status widgets in [pozitronik/steelclock-go](https://github.com/pozitronik/steelclock-go). Any such features would be implemented and reviewed for this codebase; they are not included in v0.4.0.
- **Broader hardware support:** test additional base stations and add support only when the USB/HID protocol is verified.

## Support this project

This is an **unofficial derivative** of [JerwuQu/ggoled](https://github.com/JerwuQu/ggoled). Please keep the original project's authorship and contribution in mind; if you would like to support the underlying work, consider supporting the upstream project as well.

If you still wish to support the maintenance of this derivative, an optional cryptocurrency donation is appreciated. There is no obligation to donate.

**QR code:**
<p align="left">
  <img src="https://github.com/user-attachments/assets/5bea3a8b-e33e-4165-b9ac-de72813f11b1" width="150" alt="Bağış QR Kodu" />
  <br />
</p>



## License and acknowledgements

This derivative retains the upstream **GNU General Public License v3.0**. See [`LICENSE`](LICENSE) for the full terms. Preserve applicable copyright notices and attribution when redistributing or modifying the source. This repository is not affiliated with SteelSeries.

- Original source project: [JerwuQu/ggoled](https://github.com/JerwuQu/ggoled)
- GIF studio design inspiration for possible future work: [D4rkTT/GGIF-Studio](https://github.com/D4rkTT/GGIF-Studio)
- Widget inspiration for possible future work: [pozitronik/steelclock-go](https://github.com/pozitronik/steelclock-go)
