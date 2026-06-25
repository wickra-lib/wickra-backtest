# Wickra Backtest — R

R binding for the [wickra-backtest](../../README.md) engine. Compiled C glue
calls the stable **C ABI** via `.Call`, so the results are byte-identical to the
Rust, Python, Node.js, WASM, C#, Java and Go bindings: one engine kernel behind
every language.

## Requirements

- R 4.x with a toolchain (Rtools on Windows, the standard build tools elsewhere)
- The native library `wickra_backtest` (built from the C-ABI crate)

## Build the native library

```bash
cargo build -p wickra-backtest-c           # debug   -> target/debug
cargo build -p wickra-backtest-c --release # release -> target/release
```

## Install and test

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

## Usage

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
