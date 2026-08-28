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

## Per-binding throughput — the cost of the boundary

Ten reaches, one engine. A difference between two bindings is therefore never a
difference in the backtester: it is what that language charges to cross into it,
paid once per bar on the streaming path and once per run on the batch path.

Each binding has a harness under `bindings/<lang>/benchmarks/` that runs the
[shared example strategy](examples/ema-cross.json) -- two EMAs, a crossover,
fractional sizing, taker costs, slippage and a trailing stop -- over the same
deterministic synthetic series, built from the same formula in every language.

On one development machine, 100,000 bars, median of three runs:

| Binding | streaming | ns/bar | batch | ns/bar |
|---------|----------:|-------:|------:|-------:|
| C | 6,750,000 b/s | 148 | 6,548,000 b/s | 153 |
| C# | 6,188,000 b/s | 162 | 6,315,000 b/s | 158 |
| Go | 4,621,000 b/s | 216 | 6,448,000 b/s | 155 |
| Java | 4,493,000 b/s | 223 | 5,565,000 b/s | 180 |
| WASM | 4,127,000 b/s | 242 | 4,878,000 b/s | 205 |
| Node | 3,438,000 b/s | 291 | 2,530,000 b/s | 395 |
| Python | 1,411,000 b/s | 709 | 1,486,000 b/s | 673 |
| R | 284,000 b/s | 3,527 | 6,213,000 b/s | 161 |

**C is the floor**: it calls the exported functions directly, with no
marshalling of its own, so its ~148 ns/bar is the engine plus a function call.
Every other row is that number plus what the language adds.

**The batch column collapses towards the floor.** Go, C#, R and C all land
within a few nanoseconds of each other, because batch crosses the boundary once
and spends the rest of the run inside Rust. Read down that column and you are
mostly reading the engine.

**The streaming column is the spread.** It is a call per bar, so it is where
each language's per-call cost shows: 148 ns in C, 3,527 ns in R. That is the
number to look at when deciding where to drive a live loop from, and R is why
the choice matters -- its own streaming path is 24x slower than its batch one,
entirely in the interpreter, not in the engine.

Two results worth stating because they are the opposite of what one might
assume. **Node's batch path is slower than its streaming path** (395 vs 291
ns/bar): marshalling six JavaScript arrays across napi costs more than 100,000
scalar calls. And **WASM beats the native Node binding on both paths** -- the
sandbox boundary is cheaper here than napi's.

Run them yourself; the numbers are machine-dependent and only comparable within
one machine:

```bash
cargo build -p wickra-backtest-c --release

node bindings/node/benchmarks/throughput.js
node bindings/wasm/benchmarks/throughput.cjs        # after wasm-pack --target nodejs
(cd bindings/python && python -m benchmarks.throughput)
Rscript bindings/r/benchmarks/throughput.R
(cd bindings/go && go run ./benchmarks)
dotnet run --project bindings/csharp/benchmarks -c Release
mvn -f bindings/java/benchmarks compile exec:exec
cmake -S bindings/c/benchmarks -B bindings/c/benchmarks/build   && cmake --build bindings/c/benchmarks/build --config Release   && ./bindings/c/benchmarks/build/throughput
```

Each takes `--bars` (the C one takes it positionally). The C#, Java and C
harnesses link the **release** C ABI deliberately: their test and example
projects use the debug build, and benchmarking against an unoptimised engine
measures the debug build -- which is exactly what happened on the first run of
the C# harness, off by a factor of ten until the stale debug library was
replaced.

## Versus other libraries

A reproducible harness runs the **same SMA-crossover strategy over the same
candle series** through each library and reports end-to-end backtest throughput:

```bash
cd bindings/python
python -m benchmarks.compare_libraries --size 10000 --repeat 3
```

On one development machine, best of five:

| Library | 10,000 bars | 100,000 bars | Engine |
|---------|------------:|-------------:|--------|
| wickra-backtest | ~1,810,000 b/s | ~1,440,000 b/s | event-driven, O(1) per bar (Rust) |
| vectorbt | ~2,100,000 b/s | ~8,460,000 b/s | vectorised NumPy |
| backtrader | ~11,200 b/s | ~10,400 b/s | pure-Python event loop |

**vectorbt is faster here, and increasingly so with size.** That is what
vectorisation buys and it is worth saying plainly rather than leaving the row
blank: it computes the whole array at once, in NumPy, with no per-bar event
simulation. What it does not do is run a bar at a time, which means it has no
live path — the same code cannot be pointed at a socket, and there is nothing to
be byte-identical with. This engine trades batch throughput for that property.

The comparison that does map cleanly is backtrader, which is also event-driven
and also simulates each bar: ~130x here at 10,000 bars and ~138x at 100,000.

This measures **engine-loop throughput on identical data, not identical
results**: each library models fills, sizing and costs differently, so trade
counts and P&L will not match. Numbers are from the Python binding via the
harness above, which is a different measurement from the criterion figures at the
top of this file — those run the Rust engine directly on a different strategy.

At 1,000,000 bars the harness's vectorbt path stops with `cash cannot be NaN`;
the synthetic price path drives its portfolio to zero at that length. The other
two complete.

Run the harness on your own
hardware rather than trusting a quoted figure — libraries that are not installed
are skipped, so the script always produces output.
