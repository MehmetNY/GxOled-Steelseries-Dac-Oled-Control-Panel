package main

import (
	"bytes"
	"testing"
)

func TestOriginal128x64ProfilesProduceRawFrames(t *testing.T) {
	for _, name := range []string{"matrix", "hyperspace", "starwars", "life"} {
		t.Run(name, func(t *testing.T) {
			var buf bytes.Buffer
			if err := run(name, 2, &buf); err != nil {
				t.Fatal(err)
			}
			if got := buf.Len(); got != 2048 {
				t.Fatalf("two 128x64 packed frames should be 2048 bytes, got %d", got)
			}
		})
	}
}
func TestRejectsUnknownAnimation(t *testing.T) {
	var buf bytes.Buffer
	if err := run("unknown", 1, &buf); err == nil {
		t.Fatal("unknown animation should fail")
	}
}
