# Third-party notices

GxOled Advanced is GPL-3.0 software built on [JerwuQu/ggoled](https://github.com/JerwuQu/ggoled) and, for the
animations, [pozitronik/steelclock-go](https://github.com/pozitronik/steelclock-go)
(see [`go_animations/UPSTREAM.md`](go_animations/UPSTREAM.md)). The modern settings interface (v0.5.3 Beta,
crate `gxoled_ui`) adds the following material. All of it is under licences that permit use in a GPL-3.0 program.

## Embedded in the executable

| Component | Used for | Licence | Full text |
| --- | --- | --- | --- |
| **Inter** (Regular, Medium, SemiBold, Bold; © The Inter Project Authors, <https://github.com/rsms/inter>) | UI text; the font files are embedded and are not modified | SIL Open Font License 1.1 | [`gxoled_ui/licenses/INTER-OFL-1.1.txt`](gxoled_ui/licenses/INTER-OFL-1.1.txt) |
| **Lucide** icons (© Lucide Icons and Contributors, <https://lucide.dev>) | UI icons, inlined as SVG paths | ISC | [`gxoled_ui/licenses/LUCIDE-ISC.txt`](gxoled_ui/licenses/LUCIDE-ISC.txt) |
| [`resvg`](https://github.com/linebender/resvg), `usvg`, `svgtypes`, `roxmltree`, `simplecss`, `xmlwriter` | SVG parsing and rendering | Apache-2.0 OR MIT | each crate's `LICENSE-*` |
| [`tiny-skia`](https://github.com/linebender/tiny-skia), `tiny-skia-path` | Software rasteriser | BSD-3-Clause | each crate's `LICENSE` |
| `fontdb`, `harfrust` (text shaping), `skrifa`, `read-fonts`, `font-types`, `kurbo`, `unicode-bidi`, `unicode-script`, `unicode-vo`, `png`, `flate2`, `miniz_oxide`, `fdeflate`, `bytemuck`, `base64`, `data-url`, `imagesize`, `rgb`, `log`, `memchr`, `once_cell`, `smallvec`, `tinyvec`, `slotmap`, `strict-num`, `float-cmp`, `arrayref`, `arrayvec`, `siphasher`, `polycool`, `simd-adler32`, `adler2`, `crc32fast`, `bitflags`, `cfg-if`, `pico-args` | Supporting libraries of the renderer | MIT, Apache-2.0, BSD-2-Clause, Zlib, 0BSD, Unlicense or combinations of these | each crate's licence files |

The exact dependency list and licence expressions come from `Cargo.lock` / `cargo tree -p gxoled_ui`. Inter is
bundled unchanged and is not sold on its own, as the OFL requires; its copyright and licence text are kept in the
repository and in the font files' own metadata.

## Not bundled

[GGIF-Studio](https://github.com/D4rkTT/GGIF-Studio) is only a design reference for a later phase. No code from it
is included; its licence has to be checked against GPL-3.0 before any code is taken.
