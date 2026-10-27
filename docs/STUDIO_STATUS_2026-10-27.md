# Studio development status — 2026-10-27

**Not a release.** The GitHub `stable` tag and v0.4.0 Windows Light release are preserved.

The full 107-file Studio source snapshot and an emergency 25 October Dark UI rollback ZIP were prepared in the related ChatGPT working environment. **Neither full ZIP has been uploaded into this branch.** This branch contains release automation, notes and patches only; do not label it as a complete or buildable source tree.

The user's local Windows validation of the last Studio E0382-fix source snapshot showed successful cargo check, six unit tests, and release build (two non-blocking dead-code warnings). Runtime GUI review rejected that Studio UI: revert to the prior 25 October Dark UI, while planning integration of the *actual* GGIF-Studio multi-layer editor (timeline, video/GIF sources, transform and compositing) rather than treating a few transformation controls as the full editor.

Classic GIF behavior remains the fallback. No new experimental releases should be published from an incomplete repository tree, and the stable Light binary must not be modified.

Source ZIP and patch currently require an upload channel from the local working container to GitHub; the connected GitHub Contents API exposed in this chat accepts literal file contents, but cannot accept a local sandbox ZIP path as a release asset. Use a complete source transfer before enabling release automation.
