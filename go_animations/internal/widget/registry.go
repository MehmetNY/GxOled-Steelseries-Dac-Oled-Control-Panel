package widget

import "github.com/pozitronik/steelclock-go/internal/config"

// A minimal registry: preserves the upstream widget registration API.
var registry = map[string]func(config.WidgetConfig) (Widget, error){}

func Register(name string, constructor func(config.WidgetConfig) (Widget, error)) {
	registry[name] = constructor
}
