# Fuzzing wickra-backtest

[`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html) harnesses for
the parsing and execution entry points. Fuzzing requires a nightly Rust
toolchain; on the stable toolchain the same invariants are covered by the
property tests in `crates/wickra-backtest-core/tests/properties.rs`.

## Setup

```bash
cargo install cargo-fuzz
rustup toolchain install nightly
```

## Targets

| Target | What it exercises |
| --- | --- |
| `spec_parse` | `StrategySpec::parse` over arbitrary text — malformed JSON, wrong shape, undeclared indicator references. |
| `run_json` | The unified `run_json` request bundle (spec + candles + feeds) from a single untrusted string — the full parse → validate → engine path. |
| `engine_run` | The engine over arbitrary `[open, high, low, close, volume]` candle streams with an indicator + rule strategy — NaN, ±inf, inverted bars, extreme magnitudes. |
| `fill_model` | The execution / fill model: stop-loss, take-profit, trailing stop, limit entry, leverage, maker/taker fees and slippage over arbitrary candles. |
| `data_loader` | The CSV / JSON-Lines / JSON-array candle parsers over arbitrary bytes. |

## Run

```bash
cargo +nightly fuzz run spec_parse
cargo +nightly fuzz run run_json
cargo +nightly fuzz run engine_run
cargo +nightly fuzz run fill_model
cargo +nightly fuzz run data_loader
```

Each target must run indefinitely without a crash: every input either produces
a valid result or a typed `Err`, never a panic.
