# Fuzzing Wickra Backtest

[`cargo-fuzz`](https://rust-fuzz.github.io/book/cargo-fuzz.html) harnesses for
the parsing and execution entry points. Fuzzing requires a nightly Rust
toolchain; on the stable toolchain the same invariants are covered by the
property tests in `crates/wickra-backtest-core/tests/properties.rs`.

## Setup

```bash
cargo install cargo-fuzz
rustup toolchain install nightly-2026-07-01
```

The date is the family's fuzz nightly, pinned in `ci.yml`: a rolling `nightly`
regressed with a codegen ICE unrelated to this code, so every repository moves
the date together, on purpose.

## Targets

| Target | What it exercises |
| --- | --- |
| `spec_parse` | The strategy-spec parser with arbitrary input. |
| `run_json` | The unified `run_json` entry point with arbitrary input. |
| `engine_run` | The engine over arbitrary candle sequences. |
| `fill_model` | The execution / fill model over arbitrary candle sequences. |
| `data_loader` | The candle data loaders with arbitrary input. |

## Run

```bash
# From the repository root:
cargo +nightly-2026-07-01 fuzz run --target x86_64-unknown-linux-gnu spec_parse
cargo +nightly-2026-07-01 fuzz run --target x86_64-unknown-linux-gnu run_json
cargo +nightly-2026-07-01 fuzz run --target x86_64-unknown-linux-gnu engine_run
cargo +nightly-2026-07-01 fuzz run --target x86_64-unknown-linux-gnu fill_model
cargo +nightly-2026-07-01 fuzz run --target x86_64-unknown-linux-gnu data_loader
```

Each run continues until a crash is found or it is interrupted. A short
time-boxed smoke run is what CI does:

```bash
cargo +nightly-2026-07-01 fuzz run --target x86_64-unknown-linux-gnu spec_parse -- -max_total_time=30
```

The expectation for every target is that it never panics: malformed or
adversarial input must surface as an `Err` or an in-band error, never a crash.
