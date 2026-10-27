# Source provenance

This self-contained Go module uses unmodified Go widget algorithms from
https://github.com/pozitronik/steelclock-go (source archive provided by user).

- `internal/widget/{matrix,hyperspace,gameoflife,starwarsintro}/*.go`:
  originals, including upstream unit tests.
- `internal/bitmap/{draw.go,internal_font.go,segment.go,image.go}` and
  `internal/bitmap/glyphs/*.go`: upstream original graphics and dithering.
- `internal/config/*.go`: upstream original configuration structures.
- `internal/widget/{base_widget.go,widget.go}`: upstream lifecycle contracts.
- `internal/widget/registry.go` and `internal/bitmap/border.go`:
  adapter-only code to link the minimal four-widget module without the
  unrelated web editor, music players, HID backends, or third-party services.
- `cmd/renderer/`: new GxOled bridge emitting 128x64 MSB-packed frames to
  stdout. It never opens HID/USB hardware.
- `profiles/128x64/*.json`: user's SteelClock v1.7.6.2 release profiles.

The code is GPL-3.0, matching the root GxOled LICENSE.
