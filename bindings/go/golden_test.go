package wickrabacktest

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// Every binding asserts its output against the shared golden reports
// (golden/expected/), so cross-language equality is pinned. The Go binding
// returns the engine JSON verbatim, so the match is byte-for-byte.
func TestGoldenParity(t *testing.T) {
	dir := filepath.Join("..", "..", "golden")
	entries, err := os.ReadDir(filepath.Join(dir, "cases"))
	if err != nil {
		t.Fatal(err)
	}
	type goldenCase struct {
		Name    string          `json:"name"`
		Capital float64         `json:"capital"`
		Spec    json.RawMessage `json:"spec"`
		Open    []float64       `json:"open"`
		High    []float64       `json:"high"`
		Low     []float64       `json:"low"`
		Close   []float64       `json:"close"`
		Volume  []float64       `json:"volume"`
		Time    []int64         `json:"time"`
	}
	n := 0
	for _, e := range entries {
		if filepath.Ext(e.Name()) != ".json" {
			continue
		}
		raw, err := os.ReadFile(filepath.Join(dir, "cases", e.Name()))
		if err != nil {
			t.Fatal(err)
		}
		var c goldenCase
		if err := json.Unmarshal(raw, &c); err != nil {
			t.Fatal(err)
		}
		got, err := Run(c.Open, c.High, c.Low, c.Close, c.Volume, c.Time, string(c.Spec), c.Capital)
		if err != nil {
			t.Fatalf("%s: %v", c.Name, err)
		}
		want, err := os.ReadFile(filepath.Join(dir, "expected", c.Name+".json"))
		if err != nil {
			t.Fatal(err)
		}
		if got != strings.TrimSpace(string(want)) {
			t.Errorf("%s: golden mismatch\n got: %s\nwant: %s", c.Name, got, strings.TrimSpace(string(want)))
		}
		n++
	}
	if n == 0 {
		t.Fatal("no golden cases found")
	}
}
