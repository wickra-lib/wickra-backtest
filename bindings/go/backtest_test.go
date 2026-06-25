package wickrabacktest

import (
	"encoding/json"
	"math"
	"testing"
)

const priceSpec = `{"symbol":"x","timeframe":"1h","indicators":{},` +
	`"entry":{"gt":[{"price":"close"},100]},` +
	`"exit":{"lt":[{"price":"close"},100]},` +
	`"sizing":{"type":"fixed_qty","qty":1}}`

func TestVersionIsNonEmpty(t *testing.T) {
	if Version() == "" {
		t.Fatal("version is empty")
	}
}

func TestHandComputedRoundTripMatchesEngine(t *testing.T) {
	open := []float64{100, 102, 104, 98}
	high := []float64{101, 103, 104, 98}
	low := []float64{100, 102, 99, 97}
	close := []float64{101, 103, 99, 97}
	time := []int64{0, 1, 2, 3}

	out, err := Run(open, high, low, close, nil, time, priceSpec, 1000.0)
	if err != nil {
		t.Fatalf("Run failed: %v", err)
	}

	var report struct {
		Metrics struct {
			NumTrades int `json:"num_trades"`
		} `json:"metrics"`
		Trades []struct {
			EntryPrice float64 `json:"entry_price"`
			ExitPrice  float64 `json:"exit_price"`
			Pnl        float64 `json:"pnl"`
		} `json:"trades"`
		Equity []struct {
			Equity float64 `json:"equity"`
		} `json:"equity"`
	}
	if err := json.Unmarshal([]byte(out), &report); err != nil {
		t.Fatalf("bad report json: %v", err)
	}

	if report.Metrics.NumTrades != 1 {
		t.Fatalf("num_trades = %d, want 1", report.Metrics.NumTrades)
	}
	tr := report.Trades[0]
	if math.Abs(tr.EntryPrice-102.0) > 1e-9 {
		t.Errorf("entry_price = %v, want 102", tr.EntryPrice)
	}
	if math.Abs(tr.ExitPrice-98.0) > 1e-9 {
		t.Errorf("exit_price = %v, want 98", tr.ExitPrice)
	}
	if math.Abs(tr.Pnl-(-4.0)) > 1e-9 {
		t.Errorf("pnl = %v, want -4", tr.Pnl)
	}
	last := report.Equity[len(report.Equity)-1].Equity
	if math.Abs(last-996.0) > 1e-9 {
		t.Errorf("final equity = %v, want 996", last)
	}
}

func TestInvalidSpecReturnsError(t *testing.T) {
	one := []float64{1.0}
	if _, err := RunSimple(one, one, one, one, `{"bad":true}`, 10_000.0); err == nil {
		t.Fatal("expected error for invalid spec")
	}
}
