// Command backtest runs the shared EMA-cross strategy from Go, both ways.
//
//	cd examples/go && go run .
//
// It reads the same examples/sample.csv and examples/ema-cross.json every other
// language example uses, runs the whole series at once, then feeds the same bars
// one at a time and checks that the two agree. That equality is the point of the
// library: a live loop is the streaming path with a socket in place of the file,
// so a backtest is not a separate model of the strategy.
//
// Build the C ABI first (cargo build -p wickra-backtest-c --release) and stage it
// under bindings/go/lib/<goos>_<goarch>/, as the binding's README describes.
package main

import (
	"encoding/csv"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"strconv"

	wbt "github.com/wickra-lib/wickra-backtest-go"
)

const capital = 10000.0

type bar struct {
	time                           int64
	open, high, low, close, volume float64
}

// The CSV columns are time,open,high,low,close,volume.
func loadBars(path string) ([]bar, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	rows, err := csv.NewReader(file).ReadAll()
	if err != nil {
		return nil, err
	}
	bars := make([]bar, 0, len(rows)-1)
	for _, row := range rows[1:] {
		values := make([]float64, len(row))
		for i, field := range row {
			if values[i], err = strconv.ParseFloat(field, 64); err != nil {
				return nil, fmt.Errorf("bad field %q: %w", field, err)
			}
		}
		bars = append(bars, bar{
			time:   int64(values[0]),
			open:   values[1],
			high:   values[2],
			low:    values[3],
			close:  values[4],
			volume: values[5],
		})
	}
	return bars, nil
}

func run() error {
	// Resolved from this source file so the example runs the same whatever the
	// working directory is.
	_, thisFile, _, _ := runtime.Caller(0)
	dir := filepath.Join(filepath.Dir(thisFile), "..")
	bars, err := loadBars(filepath.Join(dir, "sample.csv"))
	if err != nil {
		return fmt.Errorf("could not read sample.csv: %w", err)
	}
	spec, err := os.ReadFile(filepath.Join(dir, "ema-cross.json"))
	if err != nil {
		return fmt.Errorf("could not read ema-cross.json: %w", err)
	}

	open := make([]float64, len(bars))
	high := make([]float64, len(bars))
	low := make([]float64, len(bars))
	closes := make([]float64, len(bars))
	volume := make([]float64, len(bars))
	times := make([]int64, len(bars))
	for i, b := range bars {
		open[i], high[i], low[i] = b.open, b.high, b.low
		closes[i], volume[i], times[i] = b.close, b.volume, b.time
	}

	batch, err := wbt.Run(open, high, low, closes, volume, times, string(spec), capital)
	if err != nil {
		return err
	}

	// The same run, driven bar by bar. Replace the loop with reads from a socket
	// and this is a live strategy; nothing else about it changes.
	live, err := wbt.NewStreamingBacktest(string(spec), capital)
	if err != nil {
		return err
	}
	defer live.Close()
	for _, b := range bars {
		if err := live.Step(b.open, b.high, b.low, b.close, b.volume, b.time); err != nil {
			return err
		}
	}
	streamed, err := live.FinishJSON()
	if err != nil {
		return err
	}

	var report struct {
		Metrics struct {
			NumTrades   int     `json:"num_trades"`
			Pnl         float64 `json:"pnl"`
			ReturnPct   float64 `json:"return_pct"`
			MaxDrawdown float64 `json:"max_drawdown"`
		} `json:"metrics"`
		Equity []struct {
			Equity float64 `json:"equity"`
		} `json:"equity"`
	}
	if err := json.Unmarshal([]byte(streamed), &report); err != nil {
		return err
	}
	fmt.Printf("bars            %d\n", len(bars))
	fmt.Printf("trades          %d\n", report.Metrics.NumTrades)
	fmt.Printf("pnl             %.2f\n", report.Metrics.Pnl)
	fmt.Printf("return %%        %.2f\n", report.Metrics.ReturnPct)
	fmt.Printf("max drawdown    %.4f\n", report.Metrics.MaxDrawdown)
	fmt.Printf("final equity    %.2f\n", report.Equity[len(report.Equity)-1].Equity)

	if streamed != batch {
		return fmt.Errorf("streaming and batch disagree -- that should be impossible")
	}
	fmt.Println("\nstreaming reproduces the batch report exactly")
	return nil
}

func main() {
	if err := run(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
