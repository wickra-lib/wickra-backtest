# Examples

`ema-cross.json` is a fast/slow EMA crossover with a trailing stop;
`sample.csv` is a small synthetic OHLCV series.

Run the backtest with the `wkbt` CLI:

```bash
cargo run --bin wkbt -- run --data examples/sample.csv --spec examples/ema-cross.json
```

Write the full report plus the trade and equity streams:

```bash
cargo run --bin wkbt -- run \
  --data examples/sample.csv \
  --spec examples/ema-cross.json \
  --report report.json \
  --trades trades.jsonl \
  --equity equity.jsonl
```

The same strategy spec is just data, so it runs identically from every Wickra
language binding — the backtest values match live, by construction.

## C / C++

`c/` holds two programs that link the generated header and the compiled C ABI, so
they show what any C-capable language sees:

- `example.c` — the batch entry point: OHLCV arrays in, one report JSON out.
- `streaming.c` — the same strategy driven one bar at a time, reading the closed
  trade count and the latest equity point between bars. It also runs the same
  bars through the batch entry point and exits non-zero if the two reports
  differ, so "backtest and live are one code path" is checked from outside Rust.

Both build and run as CTest cases:

```bash
cargo build -p wickra-backtest-c --release
cmake -S examples/c -B examples/c/build
cmake --build examples/c/build --config Release
ctest --test-dir examples/c/build -C Release --output-on-failure
```

The same sources compile as C++ (`g++ -x c++ ...`); the header is `extern "C"`.

