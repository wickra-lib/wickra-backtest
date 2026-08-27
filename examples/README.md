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

## One example per language

Each directory holds one runnable program that does the same thing: read
`sample.csv` and `ema-cross.json`, run the whole series at once, then feed the
same bars one at a time and check that the two reports agree. That check is the
point -- a live loop is the streaming path with a socket in place of the file, so
a backtest is not a separate model of the strategy.

| Language | Run it | Needs |
|---|---|---|
| Python | `python examples/python/backtest.py` | `maturin develop` in `bindings/python` |
| Node | `node examples/node/backtest.js` | `npm run build` in `bindings/node` |
| WASM | `node examples/wasm/backtest.cjs` | `wasm-pack build bindings/wasm --target nodejs --out-dir pkg` |
| R | `Rscript examples/r/backtest.R` | `R CMD INSTALL bindings/r` |
| Rust | `cargo run -p wickra-backtest-examples` | nothing |
| Go | `cd examples/go && go run .` | the C ABI staged under `bindings/go/lib/<goos>_<goarch>/` |
| C# | `dotnet run --project examples/csharp` | `cargo build -p wickra-backtest-c` |
| Java | `mvn -f examples/java compile exec:exec` | `cargo build -p wickra-backtest-c`, then `mvn -f bindings/java install` |

Every one of them prints the same numbers, because they share one engine.

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

