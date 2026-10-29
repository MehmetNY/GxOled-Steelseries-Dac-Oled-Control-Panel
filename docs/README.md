# Documentation index

Everything that is not the product README lives here. Turkish guides are marked **(TR)**.

| Document | What it answers |
| --- | --- |
| [`BRANCHES-AND-RELEASES.md`](BRANCHES-AND-RELEASES.md) | Which branch/tag/release is what, how a release is made, what must not be touched. |
| [`VOLUME_OSD_HANDOFF.md`](VOLUME_OSD_HANDOFF.md) | Why GxOled used to return 4–6 s after a knob turn, how v0.5.2 fixes it, what was ruled out. |
| [`MODERN-UI-BETA.md`](MODERN-UI-BETA.md) | The v0.5.3 Beta modern settings interface: how to go back to the classic one, what is where, what is verified. |
| [`TR-WINDOWS-DERLEME.md`](TR-WINDOWS-DERLEME.md) **(TR)** | Building GxOled from source on Windows with `Validate-Windows.cmd`, incl. the "path too long" (FTK1011) failure. |
| [`images/`](images/) | Screenshots and reference artwork used by the README and the clock designs. |
| [`casio-f91w-proposed.png`](casio-f91w-proposed.png) | Proposed Casio-style clock reference. |

Related files outside `docs/`:

- [`../README.md`](../README.md) — product overview, features, contributor checklist (preserve the original UI).
- [`../CHANGELOG.md`](../CHANGELOG.md) — release history.
- [`../go_animations/UPSTREAM.md`](../go_animations/UPSTREAM.md) — steelclock-go attribution and license notes.

## Ground rules that apply to every change

1. **Rust is the only OLED writer.** Go helpers, Electron tools or a future GIF editor must never write HID reports.
2. **The original Win32 settings window is not redesigned.** New features get their own control group or window.
3. **Compiled ≠ unit-tested ≠ verified on the DAC.** Release notes say which of the three applies.
4. Tags/releases `stable`, `v0.5.0`, `v0.5.1` and published `v*` releases are not deleted, retagged or force-pushed.
