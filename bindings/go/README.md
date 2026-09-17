<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514" alt="Wickra Backtest — backtest and live are byte-identical" width="100%"></a>
</p>

[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/ci.svg)](https://github.com/wickra-lib/wickra-backtest/actions/workflows/ci.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-backtest)
[![Go module](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/go.svg)](https://pkg.go.dev/github.com/wickra-lib/wickra-backtest-go)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/license.svg)](https://github.com/wickra-lib/wickra-backtest#license)

# Wickra Backtest — Go

---

> **▶ Live demo:** run a strategy in your browser and watch the equity curve build bar by bar — **[backtest-live.wickra.org](https://backtest-live.wickra.org)** · zero backend, the same engine this repository ships, compiled to WebAssembly.

**Backtest and live — for Go. `go get github.com/wickra-lib/wickra-backtest-go` — over the C ABI via cgo, prebuilt library bundled in the module.**

Go binding for the [wickra-backtest](https://github.com/wickra-lib/wickra-backtest) engine. It calls the
stable **C ABI** through cgo, so the results are byte-identical to the Rust,
Python, Node.js, WASM, C# and Java bindings: one engine kernel behind every
language.

## Install

Use the published **`wickra-backtest-go`** module, which bundles the prebuilt C ABI
library for every platform, so `go get` + `go build` works with no extra steps
(a C compiler is still required, as the binding uses cgo):

```bash
go get github.com/wickra-lib/wickra-backtest-go
```

`wickra-backtest-go` is generated from this directory by the release pipeline: it mirrors
the Go sources, the vendored C ABI header (`include/wickra_backtest.h`) and the prebuilt
libraries under `lib/<goos>_<goarch>/`. On Linux/macOS the library path is baked
in via rpath; on Windows the DLL must be discoverable at run time (next to the
executable or on `PATH`).

### Requirements

- Go 1.23+ with cgo enabled and a C compiler (GCC/Clang/MinGW)
- The native library `wickra_backtest` (built from the C-ABI crate)

### Building from this repository (contributors)

**Build the native library.** The cgo directives in `backtest.go` link against `lib/<goos>_<goarch>/`, so build
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

**Run the tests.**

```bash
cd bindings/go
go test ./...                              # Linux/macOS (rpath resolves the lib)
PATH="$PWD/lib/windows_amd64:$PATH" go test ./...   # Windows: dll on PATH
```

## Quick start

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

The same strategy also runs one bar at a time, which is what makes a backtest and
a live loop the same code path -- swap the slice for a socket and nothing else
changes:

```go
bt, err := wickrabacktest.NewStreamingBacktest(spec, 10_000.0)
if err != nil {
	return err
}
defer bt.Close()

for _, bar := range feed {
	if err := bt.Step(bar.Open, bar.High, bar.Low, bar.Close, bar.Volume, bar.Time); err != nil {
		return err
	}
}
report, err := bt.FinishJSON()
```

`StreamingBacktest` owns a native handle, so `Close` must be called -- normally
with `defer`. `FinishJSON` also releases it, and `Close` afterwards is a no-op, so
the two compose. `StepSimple` uses zero volume and the bar index as its
timestamp, mirroring `RunSimple`. Strategies reading a side feed drive the run
with `StepJSON`, passing `{"candle": ..., "feeds": ...}` per bar.

## Benchmark

`benchmarks/` reports this binding's throughput over the shared core. It measures
the call overhead of cgo over the C ABI, not a cross-library ratio (the same Rust core runs
under every binding) — see the repository
[BENCHMARKS.md](https://github.com/wickra-lib/wickra-backtest/blob/main/BENCHMARKS.md) for the
numbers, the machine and how each harness is run.

## Documentation

The full guide, the spec reference and the API documentation live in the main
repository and the documentation site:

- **Repository:** <https://github.com/wickra-lib/wickra-backtest>
- **Docs** (guides, spec reference, cookbook): <https://backtest.wickra.org>
- **Runnable example:** [`examples/go/`](https://github.com/wickra-lib/wickra-backtest/tree/main/examples/go)

- **Repository:** <https://github.com/wickra-lib/wickra-backtest>
- **Strategy spec reference:** [STRATEGY_SPEC.md](https://github.com/wickra-lib/wickra-backtest/blob/main/docs/STRATEGY_SPEC.md)
- **Cookbook:** [COOKBOOK.md](https://github.com/wickra-lib/wickra-backtest/blob/main/docs/COOKBOOK.md)
- **Built on Wickra:** <https://github.com/wickra-lib/wickra> · <https://docs.wickra.org>

The same `StrategySpec` runs identically across Rust, Python, Node.js, WASM, C,
C++, C#, Go, Java and R — one engine kernel, byte-identical reports.

Wickra Backtest ships native bindings for Python, Node.js, WASM and Rust, plus a C ABI hub that any
C-capable language (C, C++, C#, Go, Java, R) links against — all forwarding to the
same data-driven, `unsafe`-forbidden Rust core.

## Security

Found a security issue? **Please don't open a public issue.** Report it privately
via the repository's *Security* tab (*"Report a vulnerability"*) or email
**support@wickra.org** with a subject line starting `[wickra security]`. Full
policy: <https://github.com/wickra-lib/wickra-backtest/blob/main/SECURITY.md>.

## Disclaimer

Not a trading system. Backtest results are deterministic transforms of the input
data — they are not financial advice and are not indicative of future
performance. Any use in a live trading context is at your own risk. The software
is provided **as is**, without warranty of any kind; see the license files for
the full terms.

## License

Licensed under either of [Apache-2.0](https://github.com/wickra-lib/wickra-backtest/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/wickra-lib/wickra-backtest/blob/main/LICENSE-MIT) at your option.
