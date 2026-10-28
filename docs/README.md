# Documentation index

Everything that is not the product README lives here. Turkish guides are marked **(TR)**.

| Document | What it answers |
| --- | --- |
| [`BRANCHES-AND-RELEASES.md`](BRANCHES-AND-RELEASES.md) | Which branch/tag/release is what, what must never be touched, and how a fix gets promoted into v0.5.1. |
| [`VOLUME_OSD_DIAGNOSTICS.md`](VOLUME_OSD_DIAGNOSTICS.md) | The ~6 s native-volume-OSD handoff investigation: measurements, ruled-out causes, experiments, how to read the logs. |
| [`TR-TEST-REHBERI.md`](TR-TEST-REHBERI.md) **(TR)** | Where to download the measurement package, what to double-click, what to film, what to send back. |
| [`TR-WINDOWS-DERLEME.md`](TR-WINDOWS-DERLEME.md) **(TR)** | Building GxOled from source on Windows with `Validate-Windows.cmd`, incl. the "path too long" (FTK1011) failure. |
| [`images/`](images/) | Screenshots and reference artwork used by the README and the clock designs. |
| [`casio-f91w-proposed.png`](casio-f91w-proposed.png) | Proposed Casio-style clock reference. |

Related files outside `docs/`:

- [`../README.md`](../README.md) — product overview, features, contributor checklist (preserve the original UI).
- [`../CHANGELOG.md`](../CHANGELOG.md) — release history and known issues.
- [`../go_animations/UPSTREAM.md`](../go_animations/UPSTREAM.md) — steelclock-go attribution and license notes.
- [`../diagnostics/legacy-2026-10-18/`](../diagnostics/legacy-2026-10-18/) — the preserved October 18 source used as a historical comparison build.
- [`../scripts/`](../scripts/) — `Test-VolumeOSD.ps1` and the double-click launchers of the measurement package.
- [`../scripts/analysis/analyze_osd_video.py`](../scripts/analysis/analyze_osd_video.py) — reads a phone video of the OLED plus the trace log and reports how long the native screen stayed up and which frame ended it (needs ffmpeg + numpy).

## Ground rules that apply to every change

1. **Rust is the only OLED writer.** Go helpers, Electron tools or a future GIF editor must never write HID reports.
2. **The original Win32 settings window is not redesigned.** New features get their own window/module.
3. **Compiled ≠ unit-tested ≠ verified on the DAC.** Documents and release notes must say which of the three applies, and which source commit an EXE was built from.
4. **Tags/releases `stable`, `v0.5.0`, `v0.5.1` are not deleted, retagged or force-pushed** before a fix is physically verified.
