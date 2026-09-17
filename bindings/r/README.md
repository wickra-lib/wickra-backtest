<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514-7" alt="Wickra Backtest — backtest and live are byte-identical" width="100%"></a>
</p>

[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/ci.svg)](https://github.com/wickra-lib/wickra-backtest/actions/workflows/ci.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-backtest)
[![r-universe](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/r-universe.svg)](https://wickra-lib.r-universe.dev)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/license.svg)](https://github.com/wickra-lib/wickra-backtest#license)

# Wickra Backtest — R

---

> **▶ Live demo:** run a strategy in your browser and watch the equity curve build bar by bar — **[backtest-live.wickra.org](https://backtest-live.wickra.org)** · zero backend, the same engine this repository ships, compiled to WebAssembly.

**Backtest and live — for R. `install.packages("wickrabacktest", repos = "https://wickra-lib.r-universe.dev")` — over the C ABI via `.Call`, prebuilt library fetched on install.**

R binding for the [wickra-backtest](https://github.com/wickra-lib/wickra-backtest) engine. Compiled C glue
calls the stable **C ABI** via `.Call`, so the results are byte-identical to the
Rust, Python, Node.js, WASM, C#, Java and Go bindings: one engine kernel behind
every language.

## Install

From r-universe:

```r
install.packages("wickrabacktest", repos = "https://wickra-lib.r-universe.dev")
```

The package's `configure` downloads the prebuilt C ABI library for this exact
version from the GitHub release and bundles it, so an ordinary install needs
nothing but a C toolchain (Rtools on Windows) for the thin `.Call` glue layer. To
build against a local checkout instead, point it at the header and library with
the environment variables below.

The package finds the header and native library through two environment
variables (`WKBT_INC`, `WKBT_LIB`); the library must also be reachable at run
time (on `PATH` on Windows, `LD_LIBRARY_PATH` / `DYLD_LIBRARY_PATH` elsewhere):

```bash
export WKBT_INC="$PWD/bindings/c/include"
export WKBT_LIB="$PWD/target/debug"
export PATH="$PATH:$PWD/target/debug"        # Windows: dll on PATH

R CMD INSTALL bindings/r
Rscript bindings/r/tests/run_tests.R
```

### Requirements

- R 4.x with a toolchain (Rtools on Windows, the standard build tools elsewhere)
- The native library `wickra_backtest` (built from the C-ABI crate)

### Building from this repository (contributors)

```bash
cargo build -p wickra-backtest-c           # debug   -> target/debug
cargo build -p wickra-backtest-c --release # release -> target/release
```

## Quick start

```r
library(wickrabacktest)

open  <- c(100, 102, 104, 98)
high  <- c(101, 103, 104, 98)
low   <- c(100, 102,  99, 97)
close <- c(101, 103,  99, 97)

spec <- paste0(
  '{"symbol":"x","timeframe":"1h","indicators":{},',
  '"entry":{"gt":[{"price":"close"},100]},',
  '"exit":{"lt":[{"price":"close"},100]},',
  '"sizing":{"type":"fixed_qty","qty":1}}'
)

report <- backtest_run(open, high, low, close, spec = spec, capital = 10000)
cat(report)  # BacktestReport JSON
```

- `backtest_run(open, high, low, close, volume = NULL, time = NULL, spec, capital = 10000)`
- `backtest_version()`

`volume`/`time` default to zeros / `0..n-1`. The returned JSON is the same
`BacktestReport` as every other binding; an invalid spec or mismatched inputs
raise an R error, and no panic crosses the FFI boundary.

The same strategy also runs one bar at a time, which is what makes a backtest and
a live loop the same code path -- swap the vector for a socket and nothing else
changes:

```r
bt <- backtest_stream_new(spec, capital = 10000)
for (i in seq_along(bar_close)) {
  backtest_stream_step(bt, bar_open[i], bar_high[i], bar_low[i], bar_close[i])
  cat(backtest_stream_num_trades(bt), backtest_stream_latest_equity_json(bt), "
")
}
report <- backtest_stream_finish_json(bt)
```

The handle carries a finalizer, so a run dropped without
`backtest_stream_finish_json()` or `backtest_stream_free()` still releases its
native memory. `time` defaults to the number of bars fed so far, and `volume` to
zero. Strategies reading a side feed drive the run with
`backtest_stream_step_json()`, passing `{"candle": ..., "feeds": ...}` per bar;
using a finished run raises an error.

## Benchmark

`benchmarks/` reports this binding's throughput over the shared core. It measures
the call overhead of R's native `.Call` interface over the C ABI, not a cross-library ratio (the same Rust core runs
under every binding) — see the repository
[BENCHMARKS.md](https://github.com/wickra-lib/wickra-backtest/blob/main/BENCHMARKS.md) for the
numbers, the machine and how each harness is run.

## Documentation

The full guide, the spec reference and the API documentation live in the main
repository and the documentation site:

- **Repository:** <https://github.com/wickra-lib/wickra-backtest>
- **Docs** (guides, spec reference, cookbook): <https://backtest.wickra.org>
- **Runnable example:** [`examples/r/`](https://github.com/wickra-lib/wickra-backtest/tree/main/examples/r)

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
