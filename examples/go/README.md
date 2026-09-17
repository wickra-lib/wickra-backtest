# Wickra Backtest examples — Go

Runnable Go examples for the [Wickra Backtest Go binding](../../bindings/go). The binding links against the
prebuilt C ABI library, so build and stage it once before running anything:

```bash
cargo build -p wickra-backtest-c --release
cp target/release/libwickra_backtest.so bindings/go/lib/linux_amd64/   # match your GOOS_GOARCH
```

## Run

As the CI examples job runs it, from the repository root:

```bash
cd examples/go && go run .
```
