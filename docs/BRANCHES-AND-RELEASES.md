# Branches, tags and releases

## Releases (what users can download)

| Tag | Release | Notes |
| --- | --- | --- |
| `v0.5.2` | GxOled Advanced v0.5.2 | Current. Original-layout Advanced UI; fixes the late hand-back after a volume-knob turn and adds *Display → Volume screen time* (1–5 s). Asset: `GxOled.Advanced.v0.5.2.exe`, `SHA256SUMS.txt`. |
| `v0.5.1` | GxOled Advanced v0.5.1 | Previous. Known issue: GxOled returned 4–6 s after a knob turn (fixed in v0.5.2). |
| `v0.5.0` | GxOled Advanced v0.5.0 | Original-layout Advanced UI. Immutable reference for the UI. |
| `stable` | GxOled Light v0.4.0 | Older light build (`GxOled.Light.v0.4.0.exe`, `gxoled-cli.exe`). Fallback; do not touch. |

Published `v*` releases and `stable` are never deleted, retagged or force-pushed.

## Branches

| Branch | Role |
| --- | --- |
| `main` | Source of the latest release. |
| `release/v<version>-advanced` | One per release; pushing it builds the Windows EXE (MSVC, `--locked`), records its SHA-256 and publishes the GitHub release (`.github/workflows/release.yml`). The workflow refuses to overwrite an existing release. |
| `release/v0.5.0-advanced`, `release/v0.5.1-advanced` | Source lines of those releases (references). |
| `fix/media-overlay-and-version-v050` | Older fix branch, already contained in `main`. |
| `automation/republish-exact-v0.5.0` | CI that rebuilds and replaces v0.5.0 from user-provided source. Do not re-run casually. |
| `release/v0.5.0-studio.1`, `development/studio-2026-10-27` | Early "Studio" experiments, historical. Do not merge. |
| `development/advanced-gif-studio` | Original Advanced UI + separate GIF editor experiment; candidate base for the GIF Studio module. |
| `development/studio-classic-editor-2026-10-27` | Classic-editor experiment (undo/redo work); source of ideas for the editor module. |

## How a release is made

1. Change on a branch; keep the original Win32 settings layout (only add controls, never redesign).
2. Bump `version` in the root `Cargo.toml`, the four workspace entries in `Cargo.lock` and the `GxOled Advanced vX.Y.Z` label in
   `gxoled_app/src/settings_window.rs`; add a `CHANGELOG.md` entry.
3. Push `release/vX.Y.Z-advanced`. CI runs `cargo check`, the unit tests and the release build, then creates tag `vX.Y.Z` with the EXE
   and `SHA256SUMS.txt`. Fast-forward `main` to the released commit.
4. State honestly in the release notes what was compiled, what was unit-tested and what was verified on the DAC.

## Where the future GIF Studio module fits

The GIF editor (inspired by [D4rkTT/GGIF-Studio](https://github.com/D4rkTT/GGIF-Studio)) will be a **separate editor window/module** that produces animation data for the existing Rust playback engine. It will not redesign the main window and will not write to the device. Before any code is copied from that project, its license must be checked against GPL-3.0 compatibility and attribution added next to [`go_animations/UPSTREAM.md`](../go_animations/UPSTREAM.md).
