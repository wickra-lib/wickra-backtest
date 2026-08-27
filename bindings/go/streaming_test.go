package wickrabacktest

import (
	"encoding/json"
	"fmt"
	"math"
	"strings"
	"testing"
)

// bars is the fixture the batch tests use: one trade, entered at 102 and exited
// at 98.
var bars = [4][4]float64{
	{100, 101, 100, 101},
	{102, 103, 102, 103},
	{104, 104, 99, 99},
	{98, 98, 97, 97},
}

func batchReport(t *testing.T) string {
	t.Helper()
	open := []float64{bars[0][0], bars[1][0], bars[2][0], bars[3][0]}
	high := []float64{bars[0][1], bars[1][1], bars[2][1], bars[3][1]}
	low := []float64{bars[0][2], bars[1][2], bars[2][2], bars[3][2]}
	closes := []float64{bars[0][3], bars[1][3], bars[2][3], bars[3][3]}
	out, err := RunSimple(open, high, low, closes, priceSpec, 1000.0)
	if err != nil {
		t.Fatalf("RunSimple failed: %v", err)
	}
	return out
}

// The claim worth pinning: bar-by-bar and whole-series agree exactly.
func TestStreamingReproducesTheBatchReport(t *testing.T) {
	bt, err := NewStreamingBacktest(priceSpec, 1000.0)
	if err != nil {
		t.Fatalf("NewStreamingBacktest failed: %v", err)
	}
	defer bt.Close()

	for _, b := range bars {
		if err := bt.StepSimple(b[0], b[1], b[2], b[3]); err != nil {
			t.Fatalf("StepSimple failed: %v", err)
		}
	}
	got, err := bt.FinishJSON()
	if err != nil {
		t.Fatalf("FinishJSON failed: %v", err)
	}
	if got != batchReport(t) {
		t.Error("streaming report differs from the batch report")
	}
}

func TestStepJSONMatchesTheScalarStep(t *testing.T) {
	bt, err := NewStreamingBacktest(priceSpec, 1000.0)
	if err != nil {
		t.Fatalf("NewStreamingBacktest failed: %v", err)
	}
	defer bt.Close()

	for i, b := range bars {
		doc := fmt.Sprintf(
			`{"candle":{"time":%d,"open":%g,"high":%g,"low":%g,"close":%g,"volume":0}}`,
			i, b[0], b[1], b[2], b[3])
		if err := bt.StepJSON(doc); err != nil {
			t.Fatalf("StepJSON failed: %v", err)
		}
	}
	got, err := bt.FinishJSON()
	if err != nil {
		t.Fatalf("FinishJSON failed: %v", err)
	}
	if got != batchReport(t) {
		t.Error("stepJSON report differs from the batch report")
	}
}

func TestAccessorsTrackTheRun(t *testing.T) {
	bt, err := NewStreamingBacktest(priceSpec, 1000.0)
	if err != nil {
		t.Fatalf("NewStreamingBacktest failed: %v", err)
	}
	defer bt.Close()

	latest, err := bt.LatestEquityJSON()
	if err != nil {
		t.Fatalf("LatestEquityJSON failed: %v", err)
	}
	if latest != "null" {
		t.Errorf("latest equity before the first bar = %q, want null", latest)
	}
	if n, err := bt.NumTrades(); err != nil || n != 0 {
		t.Errorf("NumTrades = %d (%v), want 0", n, err)
	}
	if bt.IsFinished() {
		t.Error("a fresh run reports itself finished")
	}

	for _, b := range bars[:3] {
		if err := bt.StepSimple(b[0], b[1], b[2], b[3]); err != nil {
			t.Fatalf("StepSimple failed: %v", err)
		}
	}

	curveJSON, err := bt.EquityJSON()
	if err != nil {
		t.Fatalf("EquityJSON failed: %v", err)
	}
	var curve []struct {
		Time   int64   `json:"time"`
		Equity float64 `json:"equity"`
	}
	if err := json.Unmarshal([]byte(curveJSON), &curve); err != nil {
		t.Fatalf("bad equity json: %v", err)
	}
	if len(curve) != 3 {
		t.Fatalf("equity length = %d, want 3", len(curve))
	}

	// Bar 2 closed below 100, which is the exit *signal*; the fill lands on the
	// next bar's open, so nothing has closed yet.
	if n, err := bt.NumTrades(); err != nil || n != 0 {
		t.Errorf("NumTrades after the signal bar = %d (%v), want 0", n, err)
	}

	if err := bt.StepSimple(bars[3][0], bars[3][1], bars[3][2], bars[3][3]); err != nil {
		t.Fatalf("StepSimple failed: %v", err)
	}
	if n, err := bt.NumTrades(); err != nil || n != 1 {
		t.Errorf("NumTrades after the fill bar = %d (%v), want 1", n, err)
	}
}

func TestStepSimpleUsesTheBarIndexAsTimestamp(t *testing.T) {
	bt, err := NewStreamingBacktest(priceSpec, 1000.0)
	if err != nil {
		t.Fatalf("NewStreamingBacktest failed: %v", err)
	}
	defer bt.Close()

	for _, b := range bars {
		if err := bt.StepSimple(b[0], b[1], b[2], b[3]); err != nil {
			t.Fatalf("StepSimple failed: %v", err)
		}
	}
	curveJSON, err := bt.EquityJSON()
	if err != nil {
		t.Fatalf("EquityJSON failed: %v", err)
	}
	var curve []struct {
		Time int64 `json:"time"`
	}
	if err := json.Unmarshal([]byte(curveJSON), &curve); err != nil {
		t.Fatalf("bad equity json: %v", err)
	}
	if len(curve) != len(bars) {
		t.Fatalf("equity length = %d, want %d", len(curve), len(bars))
	}
	for i, point := range curve {
		if point.Time != int64(i) {
			t.Errorf("equity[%d].time = %d, want %d", i, point.Time, i)
		}
	}
}

func TestAFinishedRunRefusesFurtherUse(t *testing.T) {
	bt, err := NewStreamingBacktest(priceSpec, 1000.0)
	if err != nil {
		t.Fatalf("NewStreamingBacktest failed: %v", err)
	}
	defer bt.Close()

	if err := bt.StepSimple(bars[0][0], bars[0][1], bars[0][2], bars[0][3]); err != nil {
		t.Fatalf("StepSimple failed: %v", err)
	}
	if _, err := bt.FinishJSON(); err != nil {
		t.Fatalf("FinishJSON failed: %v", err)
	}
	if !bt.IsFinished() {
		t.Fatal("run does not report itself finished")
	}

	if err := bt.StepSimple(1, 1, 1, 1); err == nil {
		t.Error("Step on a finished run returned no error")
	}
	if _, err := bt.EquityJSON(); err == nil {
		t.Error("EquityJSON on a finished run returned no error")
	}
	if _, err := bt.LatestEquityJSON(); err == nil {
		t.Error("LatestEquityJSON on a finished run returned no error")
	}
	if _, err := bt.NumTrades(); err == nil {
		t.Error("NumTrades on a finished run returned no error")
	}
	if _, err := bt.FinishJSON(); err == nil {
		t.Error("FinishJSON twice returned no error")
	}
}

func TestCloseIsIdempotent(t *testing.T) {
	bt, err := NewStreamingBacktest(priceSpec, 1000.0)
	if err != nil {
		t.Fatalf("NewStreamingBacktest failed: %v", err)
	}
	if err := bt.StepSimple(bars[0][0], bars[0][1], bars[0][2], bars[0][3]); err != nil {
		t.Fatalf("StepSimple failed: %v", err)
	}
	bt.Close()
	bt.Close()
	if !bt.IsFinished() {
		t.Error("closed run does not report itself finished")
	}
}

func TestStreamingRejectsAnInvalidSpec(t *testing.T) {
	if _, err := NewStreamingBacktest(`{"bad":true}`, 1000.0); err == nil {
		t.Fatal("expected an error for an invalid spec")
	}
}

// A pairwise indicator is undefined without its reference series, so a spec that
// reads one proves the per-bar feed actually arrives -- and it must agree with
// the batch path fed the same reference.
func TestPerBarFeedsReachAReferenceReadingStrategy(t *testing.T) {
	// A sine path, not a geometric one: constant growth means constant log
	// returns, which drives the correlation's variance to zero.
	const n = 24
	closes := make([]float64, n)
	for i := range closes {
		closes[i] = 100 + 10*math.Sin(float64(i)*0.5)
	}
	const spec = `{"symbol":"x","timeframe":"1h",` +
		`"indicators":{"corr":{"type":"PearsonCorrelation","params":[5]}},` +
		`"entry":{"gt":["corr",0.5]},"exit":{"lt":["corr",-0.5]},` +
		`"sizing":{"type":"fixed_qty","qty":1}}`

	trades := func(reportJSON string) int {
		t.Helper()
		var report struct {
			Metrics struct {
				NumTrades int `json:"num_trades"`
			} `json:"metrics"`
		}
		if err := json.Unmarshal([]byte(reportJSON), &report); err != nil {
			t.Fatalf("bad report json: %v", err)
		}
		return report.Metrics.NumTrades
	}

	fed, err := NewStreamingBacktest(spec, 1000.0)
	if err != nil {
		t.Fatalf("NewStreamingBacktest failed: %v", err)
	}
	defer fed.Close()

	var candles, reference []string
	for i, c := range closes {
		candle := fmt.Sprintf(
			`{"time":%d,"open":%g,"high":%g,"low":%g,"close":%g,"volume":0}`,
			i, c, c+1, c-1, c)
		candles = append(candles, candle)
		ref := 2 * c
		reference = append(reference, fmt.Sprintf(
			`{"time":%d,"open":%g,"high":%g,"low":%g,"close":%g,"volume":0}`,
			i, ref, ref, ref, ref))
		if err := fed.StepJSON(
			fmt.Sprintf(`{"candle":%s,"feeds":{"reference":%g}}`, candle, ref),
		); err != nil {
			t.Fatalf("StepJSON failed: %v", err)
		}
	}
	streamed, err := fed.FinishJSON()
	if err != nil {
		t.Fatalf("FinishJSON failed: %v", err)
	}

	batch, err := RunJSON(fmt.Sprintf(
		`{"spec":%s,"capital":1000,"candles":[%s],"reference":[%s]}`,
		spec, strings.Join(candles, ","), strings.Join(reference, ",")))
	if err != nil {
		t.Fatalf("RunJSON failed: %v", err)
	}
	if streamed != batch {
		t.Error("streamed report with feeds differs from the batch report")
	}
	if got := trades(streamed); got != 1 {
		t.Fatalf("fed run num_trades = %d, want 1", got)
	}

	// The feed is load-bearing: without it the correlation never resolves.
	blindRun, err := NewStreamingBacktest(spec, 1000.0)
	if err != nil {
		t.Fatalf("NewStreamingBacktest failed: %v", err)
	}
	defer blindRun.Close()
	for _, c := range closes {
		if err := blindRun.StepSimple(c, c+1, c-1, c); err != nil {
			t.Fatalf("StepSimple failed: %v", err)
		}
	}
	blind, err := blindRun.FinishJSON()
	if err != nil {
		t.Fatalf("FinishJSON failed: %v", err)
	}
	if got := trades(blind); got != 0 {
		t.Errorf("blind run num_trades = %d, want 0", got)
	}
	if blind == streamed {
		t.Error("dropping the reference feed changed nothing; the test proves nothing")
	}
}
