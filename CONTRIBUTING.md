# Contributing

Thanks for your interest in `wickra-backtest` — the streaming-native backtester
for the [Wickra](https://github.com/wickra-lib/wickra) indicator library.

## Development

```bash
cargo build --workspace
cargo test --workspace --all-features
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check
```

Run a backtest locally:

```bash
cargo run --bin wkbt -- run --data examples/sample.csv --spec examples/ema-cross.json
```

## Before opening a PR

- `cargo fmt --all` and `cargo clippy … -D warnings` must be clean (CI enforces
  this on three operating systems and the MSRV).
- Add tests. The engine is correctness-critical — prefer a hand-computed
  expectation (see `engine.rs` tests) over a smoke test.
- One logical change per PR; a clear, imperative commit message.

## Design rules

- **Strategies are data, not code.** The `StrategySpec` is JSON so the same
  strategy runs identically across every Wickra language binding. Keep the DSL
  small and serialisable.
- **No look-ahead bias.** Signals are decided on a bar's close and fill on the
  next bar's open; stop/target/trailing levels fill intrabar. Any change to the
  fill model must preserve this.
- **The engine is feed-agnostic.** It consumes a bar stream; loaders and (later)
  live feeds live outside the core so backtest and live share one engine.

## Indicators

The registry (`registry.rs`) wraps `wickra-core` indicators behind a uniform
`EvalIndicator`. New indicators are added there (and, eventually, generated from
the Wickra manifest). Multi-output indicators expose named fields referenced as
`"name.field"`.

## License

By contributing you agree that your contributions are licensed under the
project's dual `MIT OR Apache-2.0` license.
