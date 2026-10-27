// GxOled Go animation renderer. The upstream widget implementations are intentionally
// retained as Go code. This process NEVER talks to the SteelSeries device.
// Stdout is ONLY successive 1024-byte monochrome 128x64 frames in SteelClock's
// MSB-first pixel order. Stderr carries diagnostics.
package main

import (
	"bufio"
	"embed"
	"encoding/json"
	"flag"
	"fmt"
	"io"
	"os"
	"time"

	"github.com/pozitronik/steelclock-go/internal/bitmap"
	"github.com/pozitronik/steelclock-go/internal/config"
	"github.com/pozitronik/steelclock-go/internal/widget"
	"github.com/pozitronik/steelclock-go/internal/widget/gameoflife"
	"github.com/pozitronik/steelclock-go/internal/widget/hyperspace"
	"github.com/pozitronik/steelclock-go/internal/widget/matrix"
	"github.com/pozitronik/steelclock-go/internal/widget/starwarsintro"
)

//go:embed profiles/*.json
var profiles embed.FS

var names = map[string]string{
	"matrix":     "matrix.json",
	"hyperspace": "hyperspace.json",
	"starwars":   "starwars_intro.json",
	"life":       "game_of_life.json",
}

func run(name string, maxFrames int, output io.Writer) error {
	return runWithProfile(name, maxFrames, output, "")
}

func runWithProfile(name string, maxFrames int, output io.Writer, file string) error {
	filename, exists := names[name]
	if !exists {
		return fmt.Errorf("unknown animation %q", name)
	}
	data, err := profiles.ReadFile("profiles/" + filename)
	if file != "" {
		stat, statErr := os.Stat(file)
		if statErr != nil {
			return statErr
		}
		if stat.Size() > 65536 {
			return fmt.Errorf("profile file is too large")
		}
		data, err = os.ReadFile(file)
	}
	if err != nil {
		return err
	}
	var conf config.Config
	if err := json.Unmarshal(data, &conf); err != nil {
		return err
	}
	if len(conf.Widgets) != 1 || conf.Display.Width != 128 || conf.Display.Height != 64 || conf.RefreshRateMs < 10 || conf.RefreshRateMs > 1000 || conf.Widgets[0].Type != cfgType(name) {
		return fmt.Errorf("invalid 128x64 source profile")
	}
	cfg := conf.Widgets[0]
	var w widget.Widget
	switch name {
	case "matrix":
		w, err = matrix.New(cfg)
	case "hyperspace":
		w, err = hyperspace.New(cfg)
	case "starwars":
		w, err = starwarsintro.New(cfg)
	case "life":
		w, err = gameoflife.New(cfg)
	}
	if err != nil {
		return err
	}
	defer widget.StopWidget(w)

	interval := time.Duration(conf.RefreshRateMs) * time.Millisecond
	if interval < 10*time.Millisecond {
		interval = 33 * time.Millisecond
	}
	// The original SteelClock updater runs at the selected profile rate.
	ticker := time.NewTicker(interval)
	defer ticker.Stop()
	writer := bufio.NewWriterSize(output, 4096)
	var buffer = make([]byte, 1024)
	for frame := 0; maxFrames == 0 || frame < maxFrames; frame++ {
		if err := w.Update(); err != nil {
			return err
		}
		img, err := w.Render()
		if err != nil {
			return err
		}
		if img == nil {
			return fmt.Errorf("widget returned nil image")
		}
		mono, err := bitmap.ImageToBytes(img, 128, 64, buffer)
		if err != nil {
			return err
		}
		if _, err := writer.Write(mono); err != nil {
			return err
		}
		if err := writer.Flush(); err != nil {
			return err
		}
		if maxFrames == 0 || frame+1 < maxFrames {
			<-ticker.C
		}
	}
	return nil
}

func cfgType(name string) string {
	switch name {
	case "starwars":
		return "starwars_intro"
	case "life":
		return "game_of_life"
	default:
		return name
	}
}

func main() {
	profile := flag.String("animation", "", "matrix, hyperspace, starwars, or life")
	frames := flag.Int("frames", 0, "number of frames (0 = run until stopped)")
	custom := flag.String("profile-file", "", "user-customized 128x64 profile JSON")
	flag.Parse()
	if err := runWithProfile(*profile, *frames, os.Stdout, *custom); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
