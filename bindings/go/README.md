# Wickra Backtest — Go

Go binding for the [wickra-backtest](../../README.md) engine. It calls the
stable **C ABI** through cgo, so the results are byte-identical to the Rust,
Python, Node.js, WASM, C# and Java bindings: one engine kernel behind every
language.

## Requirements

- Go 1.23+ with cgo enabled and a C compiler (GCC/Clang/MinGW)
- The native library `wickra_backtest` (built from the C-ABI crate)

## Build the native library

The cgo directives in `backtest.go` link against `lib/<goos>_<goarch>/`, so build
the C ABI crate and stage the library into the directory for your platform:

```bash
cargo build -p wickra-backtest-c --release
# Linux x64:   cp target/release/libwickra_backtest.so    bindings/go/lib/linux_amd64/
# macOS arm64: cp target/release/libwickra_backtest.dylib bindings/go/lib/darwin_arm64/
# Windows x64: cp target/release/wickra_backtest.dll      bindings/go/lib/windows_amd64/
```

On Linux/macOS the library path is baked in via rpath. On Windows the DLL must be
discoverable at run time (next to the executable or on `PATH`).

The published [`wickra-backtest-go`](https://github.com/wickra-lib/wickra-backtest-go)
module ships these prebuilt libraries for every platform, so end users only run
`go get` — the staging above is for contributors building from this directory.

## Run the tests

```bash
cd bindings/go
go test ./...                              # Linux/macOS (rpath resolves the lib)
PATH="$PWD/lib/windows_amd64:$PATH" go test ./...   # Windows: dll on PATH
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

## Documentation

- **Repository:** <https://github.com/wickra-lib/wickra-backtest>
- **Strategy spec reference:** [STRATEGY_SPEC.md](../../docs/STRATEGY_SPEC.md)
- **Cookbook:** [COOKBOOK.md](../../docs/COOKBOOK.md)
- **Built on Wickra:** <https://github.com/wickra-lib/wickra> · <https://docs.wickra.org>

The same `StrategySpec` runs identically across Rust, Python, Node.js, WASM, C,
C++, C#, Go, Java and R — one engine kernel, byte-identical reports.

## Security

Found a security issue? **Please don't open a public issue.** Report it privately
via the repository's *Security* tab (*"Report a vulnerability"*) or email
**support@wickra.org**. Full policy:
<https://github.com/wickra-lib/wickra-backtest/blob/main/SECURITY.md>.

## Disclaimer

Not a trading system. Backtest results are deterministic transforms of the input
data — they are not financial advice and are not indicative of future
performance. Any use in a live trading context is at your own risk. Provided
**as is**, without warranty of any kind.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache-2.0](../../LICENSE-APACHE) at your option.
