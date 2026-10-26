# GxOled development and automatic releases

**Do not overwrite the existing [GxOled v0.4.0 stable (Light)](https://github.com/MehmetNY/GxOled-v0.4.0/releases/tag/stable) release or move the `stable` tag.** It remains available with its original Windows binaries.

## Development snapshot — 27 October 2026

This branch prepares automated release builds for the newer UI and GIF Studio work. The latest local source snapshot is `GxOled-Studio-2026-10-27-source.zip` (experimental; Windows compile and device test pending). **This branch currently does not contain the complete source snapshot**; do not present it as a published Studio build.

The latest snapshot keeps Classic GIF as the default/fallback, adds a separate Studio GIF processing path (128x64, geometry, tone, dithering and playback controls), introduces Studio Orange/Cyan/Violet accent styles, and retains the SteelClock-Go animation helper. It is not yet the complete GGIF-Studio multilayer/video/timeline editor.

## Future GitHub release workflow

1. Place all files of the new complete source snapshot at repository root on a **new** branch named `release/v0.5.0-studio.1` (then `release/v0.5.0-studio.2`, etc.).
2. Include `.github/workflows/release-windows.yml` in that branch.
3. Each release branch push automatically checks Rust, builds the original Go animation helper, runs tests, compiles the Windows app, bundles source and publishes a **new GitHub prerelease** with the EXE. Builds or tests that fail will not publish.
4. The workflow refuses to overwrite an existing release tag. Once a version is published, create a different version for later changes.

These are *automation instructions*, not a claim that the October Studio release has already been built or published.

See the existing stable release for the current downloadable Light build.
