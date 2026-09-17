# Wickra Backtest examples — R

Runnable R examples for the [Wickra Backtest R binding](../../bindings/r). The package compiles a thin
`.Call` glue layer against the C ABI library, so build the library and install
the package first (the CI examples job does exactly this):

```bash
cargo build -p wickra-backtest-c --release
R CMD INSTALL bindings/r
```

## Run

```bash
Rscript examples/r/<example>.R
```
