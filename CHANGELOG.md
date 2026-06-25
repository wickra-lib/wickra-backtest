# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Workspace scaffold: `wickra-backtest-core` (engine core), `wickra-backtest-data`
  (loaders) and the `wickra-backtest` facade, depending on `wickra-core`.
- `BacktestError` error type and crate `version()` helpers.
- `Candle` input type with conversion to `wickra-core` and derived prices.
- The data-driven `StrategySpec` DSL (`spec.rs`): indicators, entry/exit/short
  conditions, the `Operand`/`Condition` expression trees, sizing, costs,
  slippage, risk and execution models, with `parse`/`validate` and structural
  reference checking.
- Engine MVP: a minimal hand-written indicator registry (`Sma`/`Ema`/`Rsi`/`Atr`)
  behind a uniform `EvalIndicator`, rule evaluation (`rules.rs`), long-only
  portfolio accounting (`portfolio.rs`), performance metrics (`metrics.rs`), the
  `BacktestReport` (`report.rs`) and the event-driven loop (`engine.rs`) with
  `run`/`run_with_capital`. Look-ahead bias is structurally prevented
  (signal-on-close, fill-on-next-open); fixed fee + fixed-bps slippage and
  close-based stop/target are modelled.

- Registry expansion: more single-output indicators (`Wma`, `Dema`, `Tema`,
  `Hma`, `Roc`, `Mom`) and multi-output indicators (`Macd`, `Bollinger`,
  `Stochastic`) whose named fields are referenced in the spec as `"name.field"`
  (e.g. `macd.signal`, `bb.upper`). `EvalIndicator` now exposes named fields.

### Changed

- Execution depth: the portfolio is now signed (long **and** short positions),
  and stop-loss / take-profit are checked **intrabar** against each bar's OHLC and
  fill at the level (conservative stop-before-target ordering) instead of at the
  next close. Short entries/exits use `short_entry` / `short_exit`.

[Unreleased]: https://github.com/wickra-lib/wickra-backtest/commits/main
