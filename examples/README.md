# Wickra Backtest examples

`ema-cross.json` is a fast/slow EMA crossover with a trailing stop;
`sample.csv` is a small synthetic OHLCV series.

## Rust — `examples/rust/`

| Example | What it does |
| --- | --- |
| `src/main.rs` | Run the shared EMA-cross strategy from Rust, both ways. |

## C / C++ — `examples/c/`

Build the library first (`cargo build -p wickra-backtest-c --release`), then build and run
the examples via CMake, as the CI C ABI job does:

```bash
cmake -S examples/c -B examples/c/build
cmake --build examples/c/build --config Release
ctest --test-dir examples/c/build -C Release --output-on-failure
```

| Example | What it does |
| --- | --- |
| `cpp_smoke.cpp` | C++ example for the wickra-backtest C ABI, through the optional RAII wrapper (`wickra_backtest.hpp`). |
| `example.c` | Minimal C example for the wickra-backtest C ABI. |
| `example_cpp.cpp` | C++ build of the minimal C example. |
| `streaming.c` | Streaming example for the wickra-backtest C ABI. |
| `streaming_cpp.cpp` | C++ build of the streaming example, from the same source as the C target. |

## C# — `examples/csharp/`

| Example | What it does |
| --- | --- |
| `Program.cs` | Run the shared EMA-cross strategy from C#, both ways. |

## Go — `examples/go/`

## R — `examples/r/`

## Java — `examples/java/`

## Python — `examples/python/`

## Node.js — `examples/node/`

## WASM — `examples/wasm/`

Build the WASM package, serve the repository root, and open the page in a browser;
the module script inside it is what runs (CI parses it with `node --check`):

```bash
wasm-pack build bindings/wasm --target web
python -m http.server 8000     # then open http://localhost:8000/examples/wasm/
```

## Example datasets

The examples are self-contained: the spec and the input are inline, so there is
no shared `data/` directory to load. The cross-language golden fixtures, which
every binding is checked against byte for byte, live in [`../golden/`](../golden).

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

Every one of them prints the same numbers, because they share one engine --
and every one of them exits non-zero if its two reports disagree. CI runs all
ten, each in the job that has just built that language's binding, so an
example that stops working fails the build rather than waiting for a reader
to try it.
