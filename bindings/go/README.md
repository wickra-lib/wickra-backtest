# Wickra Backtest — Go

Go binding for the [wickra-backtest](../../README.md) engine. It calls the
stable **C ABI** through cgo, so the results are byte-identical to the Rust,
Python, Node.js, WASM, C# and Java bindings: one engine kernel behind every
language.

## Requirements

- Go 1.23+ with cgo enabled and a C compiler (GCC/Clang/MinGW)
- The native library `wickra_backtest` (built from the C-ABI crate)

## Build the native library

```bash
cargo build -p wickra-backtest-c           # debug   -> target/debug
cargo build -p wickra-backtest-c --release # release -> target/release
```

The cgo directives in `backtest.go` link against `target/debug`. At run time the
library must be reachable: add the target directory to `PATH` (Windows),
`LD_LIBRARY_PATH` (Linux) or `DYLD_LIBRARY_PATH` (macOS).

## Run the tests

```bash
cd bindings/go
PATH="$PATH:$(cd ../../target/debug && pwd)" go test ./...   # Windows: dll on PATH
```

## Usage

```go
import wickrabacktest "github.com/wickra-lib/wickra-backtest-go"

open  := []float64{100, 102, 104, 98}
high  := []float64{101, 103, 104, 98}
low   := []float64{100, 102, 99, 97}
close := []float64{101, 103, 99, 97}

spec := `{"symbol":"x","timeframe":"1h","indicators":{},` +
    `"entry":{"gt":[{"price":"close"},100]},` +
    `"exit":{"lt":[{"price":"close"},100]},` +
    `"sizing":{"type":"fixed_qty","qty":1}}`

report, err := wickrabacktest.RunSimple(open, high, low, close, spec, 10_000)
if err != nil {
    log.Fatal(err)
}
fmt.Println(report) // BacktestReport JSON
```

- `RunSimple(open, high, low, close, spec, capital)` — zero volume, `0..n` timestamps
- `Run(open, high, low, close, volume, time, spec, capital)` — full control (`volume`/`time` may be `nil`)

The returned JSON is the same `BacktestReport` as every other binding. An invalid
spec or mismatched inputs return an `error` wrapping the engine message; no panic
crosses the FFI boundary.
