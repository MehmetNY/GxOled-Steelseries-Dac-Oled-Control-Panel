# Volume-screen hand-back (what happens when you turn the DAC knob)

Short technical record of the v0.5.0–v0.5.1 "GxOled returns 4–6 s late" bug and of the v0.5.2 fix.
Measured 2026-10-28/29 on a Nova Pro Wireless Xbox base station (USB `1038:12E5`) with a 60 fps phone video of the OLED
aligned to a timestamped frame log. The investigation tooling (trace log, frame counter overlay, experiment lab, video analyzer,
packaged test builds) is not part of the product any more; this document keeps what it established.

## Behaviour of the base station

1. Turning the knob makes the firmware draw its own screen (home screen, or the "VOLUME" bar in the simple layout) and send
   an unsolicited dial report (`VolumeDial`).
2. **For roughly 3.5 s after the last dial report the firmware discards every host `0x93` frame.** Frames are neither shown
   late nor buffered. The first frame written *after* that window replaces the native screen at once.
3. GxOled's hold only decides when it starts writing again. Without anything else, the first frame at 2.5 s was discarded and
   the next one (up to a second later with a 1 Hz heartbeat) made it through: 3.9–4.5 s observed, up to 6 s in longer turns.
4. The HID transport was ruled out: the October 18 build and v0.5.1 send byte-identical `0x93` reports, and a frame written 200 ms
   after a dial report completed in about 3 ms (no error) but was never shown.

## The fix (v0.5.2)

When the hold ends GxOled sends `0x95` ("return to the SteelSeries UI") on the OLED HID collection, waits 40 ms and forces
a frame. The frame is accepted immediately. The command is listed in the community-documented opcode table of this device family
(loteran/Arctis-Sound-Manager) and changes no persistent setting.

| Situation (2026-10-29 lab, 25 native-screen episodes) | Native screen visible after the last dial report |
| --- | --- |
| Plain frames after a 2.5 s hold (control) | 3.9–4.5 s (start state), 3.6 s (Sonar flag on) |
| `0x95` at the end of a 2.5 s hold | 2.62, 2.63, 2.63, 2.65, 2.62 s (5 of 5, three different device states) |
| `0x95` before every dial report and no hold | native screen only flashes for 0.15–0.3 s (firmware volume display effectively suppressed) |

The selectable time (1–5 s) only moves the moment of the hand-back; the owner confirmed the default 2.5 s on hardware. The other
times rely on the same mechanism (in the lab, frames sent right after `0x95` were shown within about 0.3 s of a dial report) and were not
filmed one by one.

## Things that were tried or observed and are *not* used

- **Sonar-present flag (`0x8d`)**: with it set the lock was ≈ 3.5 s, with it cleared a frame at 2.5 s was already accepted. It is a
  persistent device setting that SteelSeries GG manages, so GxOled does not touch it; `0x95` works with either value.
- **Home screen type (`0x89`)**: changes the native screen layout (0 = detailed, 1 = simple); it does not change the timing.
  An earlier lab run left a test device on "simple"; it can be set back in the DAC's own menu.
- **Higher frame rate / pulse during the lock**: frames are discarded, so it cannot help.
- **Info HID collection writes** (settings query `0x80`, `0xb0`, `0x20`) fail on Windows for PID `12E5` with
  `WriteFile: (0x00000001)`; only OLED-collection writes work. The device's own settings therefore cannot be read back.

## How to re-measure

Film the OLED at 60 fps from a fixed phone, turn the knob a few clicks, let go, and read the time until GxOled's clock is back
(frame by frame). It should be the chosen hold plus about 0.1 s. The trace that earlier builds wrote is intentionally gone;
to debug a regression, log timestamps around the `0x95` call in `gxoled_draw::run_draw_device_thread`.
