# Benchmarks

The engine is event-driven and **O(1) per bar** — indicators update in constant
time (the same `wickra-core` kernels that power live Wickra), so backtest time
scales linearly with the number of bars, not with the indicator window.

## Throughput

Measured with [criterion](crates/wickra-backtest-bench/benches/backtest.rs) on a
deterministic synthetic OHLCV series, running a fast/slow **EMA crossover with a
trailing stop** (two indicators, costs and slippage modelled):

| Bars    | Time (median) | Throughput        |
|---------|---------------|-------------------|
| 10,000  | ~3.7 ms       | ~2.7M bars/second |
| 100,000 | ~60 ms        | ~1.7M bars/second |

Reproduce:

```bash
cargo bench -p wickra-backtest-bench
```

## What that means

At ~1.7M bars/second a single core backtests:

- **1 year of 1-minute bars** (~525k) in **~0.3 s**
- **10 years of 1-minute bars** (~5.3M) in **~3 s**

Whole histories per second — vs the recompute-on-every-tick approach of
pandas-based backtesters, where each indicator is recomputed over its full
window on every bar.

## Honest caveats

- Numbers are from a development machine in the release profile; absolute timings
  vary by hardware. Throughput also drops with more indicators and more complex
  rule trees.
- The engine retains only as many bars as the strategy's rules can reach back
  (the deepest `prev` / `rising` / `falling` / cross, or the vol-target lookback),
  so memory is `O(lookback × indicators)` and does not grow with the length of the
  run.
- The equity curve and trade list do still grow with it, because they are the
  report: `finish` computes every metric over the whole series. An equity point is
  16 bytes, so a year of 1-minute bars costs about 8 MB, and trades are far
  sparser. A live loop that reads `latestEquity` each bar and persists it
  externally never needs the accumulated copy -- but the engine cannot discard it
  and still report on it, so it keeps it. Ending the session with `finish` is what
  releases it; restarting one costs the indicators their warmup.
- This is a single-symbol, single-strategy micro-benchmark. It measures the
  engine, not a realistic multi-asset optimisation workload.

## Versus other libraries

A reproducible harness runs the **same SMA-crossover strategy over the same
candle series** through each library and reports end-to-end backtest throughput:

```bash
cd bindings/python
python -m benchmarks.compare_libraries --size 10000 --repeat 3
```

On one development machine over 10,000 bars (best of 3):

| Library | bars/second | Notes |
|---------|-------------|-------|
| **wickra-backtest** | ~564,000 | O(1)-per-bar streaming engine (Rust) |
| backtrader | ~4,400 | pure-Python event loop (~128× slower here) |
| vectorbt | — | vectorised NumPy; not installed in this run (skipped automatically) |

This measures **engine-loop throughput on identical data, not identical
results**: each library models fills, sizing and costs differently, so trade
counts and P&L will not match. backtrader is the closest comparison because it is
also an event-driven engine; vectorbt is vectorised (fast, but it recomputes over
the whole array and is not a streaming/live engine). Run the harness on your own
hardware rather than trusting a quoted figure — libraries that are not installed
are skipped, so the script always produces output.
