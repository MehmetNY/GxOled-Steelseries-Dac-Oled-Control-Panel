package main

import (
	"bytes"
	"os"
	"path/filepath"
	"testing"
)

func TestCustomProfileUsesOriginalWidgetAndValidatesDimensions(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "matrix.json")
	original, err := profiles.ReadFile("profiles/matrix.json")
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, original, 0600); err != nil {
		t.Fatal(err)
	}
	var buffer bytes.Buffer
	if err := runWithProfile("matrix", 2, &buffer, path); err != nil {
		t.Fatal(err)
	}
	if buffer.Len() != 2*1024 {
		t.Fatalf("got %d bytes", buffer.Len())
	}
	invalid := bytes.Replace(original, []byte(`"width": 128`), []byte(`"width": 40`), 1)
	if err := os.WriteFile(path, invalid, 0600); err != nil {
		t.Fatal(err)
	}
	if err := runWithProfile("matrix", 1, &buffer, path); err == nil {
		t.Fatal("expected 128x64 check to reject invalid file")
	}
}
