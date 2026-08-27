package wickrabacktest

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// Driving each shared case one bar at a time must reproduce the same canonical
// report (golden/expected/) the batch entry point produces. TestGoldenParity
// pins the batch side; this pins that streaming did not drift away from it.
func TestStreamingGoldenParity(t *testing.T) {
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

	seen := 0
	for _, entry := range entries {
		if !strings.HasSuffix(entry.Name(), ".json") {
			continue
		}
		raw, err := os.ReadFile(filepath.Join(dir, "cases", entry.Name()))
		if err != nil {
			t.Fatal(err)
		}
		var c goldenCase
		if err := json.Unmarshal(raw, &c); err != nil {
			t.Fatalf("bad case %s: %v", entry.Name(), err)
		}

		bt, err := NewStreamingBacktest(string(c.Spec), c.Capital)
		if err != nil {
			t.Fatalf("%s: NewStreamingBacktest failed: %v", c.Name, err)
		}
		for i := range c.Close {
			if err := bt.Step(c.Open[i], c.High[i], c.Low[i], c.Close[i],
				c.Volume[i], c.Time[i]); err != nil {
				bt.Close()
				t.Fatalf("%s: Step failed at bar %d: %v", c.Name, i, err)
			}
		}
		got, err := bt.FinishJSON()
		bt.Close()
		if err != nil {
			t.Fatalf("%s: FinishJSON failed: %v", c.Name, err)
		}

		want, err := os.ReadFile(filepath.Join(dir, "expected", c.Name+".json"))
		if err != nil {
			t.Fatal(err)
		}
		if got != strings.TrimSpace(string(want)) {
			t.Errorf("streaming mismatch for %s", c.Name)
		}
		seen++
	}
	if seen == 0 {
		t.Fatal("no golden cases found")
	}
}
