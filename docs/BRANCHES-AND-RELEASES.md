# Branches, tags and releases

State as read from GitHub on **2026-10-28**. "Ahead/behind" is relative to `main` (`b0b1348`, the v0.5.1 source).

## Releases (what users can download)

| Tag | Release | Notes |
| --- | --- | --- |
| `stable` (`f3436c2`) | GxOled Light v0.4.0 stable | Older light build (`GxOled.Light.v0.4.0.exe`, `gxoled-cli.exe`). Fallback; do not touch. |
| `v0.5.0` (`0f6d662`) | GxOled Advanced v0.5.0 | Original-layout Advanced UI. Assets: EXE, Windows ZIP, source ZIP, `SHA256SUMS.txt`. Immutable reference for the UI. |
| `v0.5.1` (`b0b1348` = `main`) | GxOled Advanced v0.5.1 | Original layout + the first volume-OSD handoff correction. Asset: `GxOled.Advanced.v0.5.1.exe`, `SHA256SUMS.txt`. **Known issue:** native volume display still lasts ~6 s (see [`VOLUME_OSD_DIAGNOSTICS.md`](VOLUME_OSD_DIAGNOSTICS.md)). |

Measurement packages are published as **pre-releases whose tag does not start with `v`** (for example `diag-osd-stamp-<sha7>`). They are never marked "latest" and they never replace the releases above.

## Branches

| Branch | Role | Position vs `main` | Action |
| --- | --- | --- | --- |
| `main` | Source of v0.5.1. | — | Keep untouched until a fix is verified on a physical DAC. |
| `release/v0.5.1-advanced` | Same commit as `main`. | identical | Keep (release reference). |
| `fix/media-overlay-and-version-v050` | Older fix branch, already contained in `main`. | 0 ahead / 1 behind | Keep; can be archived later. |
| `release/v0.5.0-advanced` | Source line of v0.5.0. | 4 ahead / 7 behind | Keep (v0.5.0 reference). |
| `automation/republish-exact-v0.5.0` | Per its last commit message: CI that rebuilds and replaces v0.5.0 from the user-provided source. | 3 ahead / 7 behind | Keep; do not re-run it casually, its purpose is to replace a release. |
| `release/v0.5.0-studio.1` | Early "Studio" import attempt. | 4 ahead / 7 behind | Historical; do not merge. |
| `development/advanced-gif-studio` | Original Advanced UI + separate GIF editor experiment. | 3 ahead / 7 behind | To be evaluated as a base for the future GIF Studio module. |
| `development/studio-2026-10-27` | Studio experiment, marked incomplete. | 6 ahead / 7 behind | Historical. |
| `development/studio-classic-editor-2026-10-27` | Classic-editor experiment (undo/redo work). | 3 ahead / 7 behind | Source of ideas for the editor module. |
| `diagnostics/volume-osd-timing` | First diagnostic line: trace log, hold/no-hold/pulse/legacy modes, October 18 comparison build. | 11 ahead / 0 behind | Fully contained in the branch below; keep for history. |
| `diagnostics/osd-frame-stamp` | **Active work**: frame-stamp measurement, single-instance guard, short-path Windows build script, measurement package, docs. | ahead of `main` | Work happens here until a fix is verified. |

## How a fix reaches v0.5.1

1. Measure on the DAC (see [`TR-TEST-REHBERI.md`](TR-TEST-REHBERI.md)). Keep the logs and the video notes.
2. Implement the smallest change that the measurement supports, on a branch, with a regression test.
3. CI builds an MSVC EXE; the workflow records the source commit and SHA-256.
4. **Physical verification:** the same camera protocol, 5+ repetitions, native OSD → GxOled return time reported honestly.
5. Only after that is the v0.5.1 asset replaced (explicitly pre-authorised by the owner). `stable` and `v0.5.0` are never deleted or retagged.

## Where the future GIF Studio module fits

The GIF editor (inspired by [D4rkTT/GGIF-Studio](https://github.com/D4rkTT/GGIF-Studio)) will be a **separate editor window/module** that produces animation data for the existing Rust playback engine. It will not redesign the main window and will not write to the device. Before any code is copied from that project, its license must be checked against GPL-3.0 compatibility and attribution added next to [`go_animations/UPSTREAM.md`](../go_animations/UPSTREAM.md).
