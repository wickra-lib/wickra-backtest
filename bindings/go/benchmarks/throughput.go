// Command throughput measures what crossing the boundary costs in the
// wickra-backtest Go binding.
//
// Every reach in this repository runs the same Rust engine, so a difference
// between two bindings is not a difference in the backtester -- it is the price
// of that language's FFI, paid once per bar on the streaming path and once per
// run on the batch path. That is the number worth knowing before choosing where
// to drive a live loop from.
//
// The strategy is examples/ema-cross.json: two EMAs, a crossover, fractional
// sizing, taker costs, slippage and a trailing stop. A realistic bar rather
// than an empty one, so the figure includes the engine work a real strategy
// does.
//
// Build the C ABI first and stage it under bindings/go/lib/<goos>_<goarch>/,
// as the binding's README describes, then:
//
//	cd bindings/go && go run ./benchmarks                  # 200k bars
//	cd bindings/go && go run ./benchmarks --bars 1000000
package main

import (
	"flag"
	"fmt"
	"log"
	"math"
	"sort"
	"time"

	wbt "github.com/wickra-lib/wickra-backtest-go"
)

const spec = `{"symbol":"BTCUSDT","timeframe":"1h",` +
	`"indicators":{"ema_fast":{"type":"Ema","params":[5]},` +
	`"ema_slow":{"type":"Ema","params":[15]}},` +
	`"entry":{"cross_above":["ema_fast","ema_slow"]},` +
	`"exit":{"cross_below":["ema_fast","ema_slow"]},` +
	`"sizing":{"type":"fixed_fraction","fraction":0.95},` +
	`"costs":{"taker_bps":5,"slippage":{"type":"fixed_bps","bps":2}},` +
	`"risk":{"trailing_stop_pct":5.0}}`

const capital = 10000.0

// series builds the deterministic synthetic OHLCV every binding's harness uses.
// No RNG, so two runs are comparable and so are two languages.
func series(bars int) (open, high, low, closes, volume []float64, times []int64) {
	open = make([]float64, bars)
	high = make([]float64, bars)
	low = make([]float64, bars)
	closes = make([]float64, bars)
	volume = make([]float64, bars)
	times = make([]int64, bars)
	for i := 0; i < bars; i++ {
		mid := 100 + math.Sin(float64(i)*0.001)*20 + float64(i)*1e-4
		closes[i] = mid + math.Sin(float64(i)*0.05)*2
		if i == 0 {
			open[i] = closes[i]
		} else {
			open[i] = closes[i-1]
		}
		high[i] = math.Max(open[i], closes[i]) + 1.5
		low[i] = math.Min(open[i], closes[i]) - 1.5
		volume[i] = 1000 + float64(i%97)*13
		times[i] = int64(i)
	}
	return
}

// medianDuration runs fn once to warm up, then reports the median of reps runs.
func medianDuration(fn func(), reps int) time.Duration {
	fn()
	samples := make([]time.Duration, reps)
	for i := range samples {
		start := time.Now()
		fn()
		samples[i] = time.Since(start)
	}
	sort.Slice(samples, func(a, b int) bool { return samples[a] < samples[b] })
	return samples[len(samples)/2]
}

func main() {
	bars := flag.Int("bars", 200000, "bars per run")
	flag.Parse()
	if *bars < 1000 {
		log.Fatal("--bars must be at least 1000")
	}

	open, high, low, closes, volume, times := series(*bars)

	streaming := medianDuration(func() {
		live, err := wbt.NewStreamingBacktest(spec, capital)
		if err != nil {
			log.Fatalf("could not start the run: %v", err)
		}
		defer live.Close()
		for i := 0; i < *bars; i++ {
			if err := live.Step(open[i], high[i], low[i], closes[i], volume[i], times[i]); err != nil {
				log.Fatalf("bar %d rejected: %v", i, err)
			}
		}
		if _, err := live.FinishJSON(); err != nil {
			log.Fatalf("could not finish the run: %v", err)
		}
	}, 3)

	batch := medianDuration(func() {
		if _, err := wbt.Run(open, high, low, closes, volume, times, spec, capital); err != nil {
			log.Fatalf("batch run failed: %v", err)
		}
	}, 3)

	fmt.Printf("wickra-backtest Go throughput — %s bars (median of 3 runs)\n\n", commas(int64(*bars)))
	fmt.Printf("%-14s%16s%12s\n", "path", "bars/sec", "ns/bar")
	fmt.Println("------------------------------------------")
	for _, row := range []struct {
		name string
		took time.Duration
	}{{"streaming", streaming}, {"batch", batch}} {
		perSecond := float64(*bars) / row.took.Seconds()
		nsPerBar := float64(row.took.Nanoseconds()) / float64(*bars)
		fmt.Printf("%-14s%16s%12s\n", row.name, commas(int64(perSecond)), commas(int64(nsPerBar)))
	}
	fmt.Print("\nStreaming crosses the boundary once per bar, with scalars; batch crosses it\n",
		"once per run and hands over six slices. cgo charges per call, so which of\n",
		"the two wins is a property of the language, not of the engine behind both.\n",
		"Machine-dependent — compare bindings on one machine, not across machines.\n")
}

// commas renders n with thousands separators, which the standard library does
// not offer and which one extra dependency would not justify.
func commas(n int64) string {
	digits := fmt.Sprintf("%d", n)
	sign := ""
	if digits[0] == '-' {
		sign, digits = "-", digits[1:]
	}
	out := make([]byte, 0, len(digits)+len(digits)/3)
	for i, c := range []byte(digits) {
		if i > 0 && (len(digits)-i)%3 == 0 {
			out = append(out, ',')
		}
		out = append(out, c)
	}
	return sign + string(out)
}
