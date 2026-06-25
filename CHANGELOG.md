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

- Python binding (`wickra-backtest`, PyO3 + maturin, abi3-py39): `wickra_backtest.run(open, high, low, close, …, spec=…)`
  runs a strategy spec over OHLCV arrays (lists or NumPy) and returns the report
  as a dict. A CI job builds and tests it on Linux/macOS/Windows.
- Data loaders (`wickra-backtest-data`): CSV (`time,open,high,low,close[,volume]`,
  optional header), JSON Lines and JSON-array candle files, dispatched by extension.
- `wkbt` command-line backtester (`wickra-backtest-cli`): `wkbt run --data … --spec …
  [--capital N] [--report …] [--trades …] [--equity …]` prints a metrics summary
  and optionally writes the report (JSON) and trade/equity streams (JSON Lines).
- Registry expansion to ~26 indicators: single-output (`Wma`, `Dema`, `Tema`,
  `Hma`, `Roc`, `Mom`, `Cmo`, `Trix`, `Trima`, `Kama`, `Cci`, `WilliamsR`,
  `Mfi`, `Vwap`, `Obv`) and multi-output (`Macd`, `Bollinger`, `Stochastic`,
  `Adx`, `Aroon`, `Keltner`, `Donchian`) whose named fields are referenced in
  the spec as `"name.field"` (e.g. `macd.signal`, `bb.upper`, `adx.plus_di`).
  `EvalIndicator` now exposes named fields.

- Trailing stop (`trailing_stop_pct`): exits intrabar when price retraces past
  the trailed favourable extreme since entry (side-aware).
- `examples/` (an EMA-crossover spec + a sample CSV) and a README quickstart.
- A criterion throughput benchmark (`wickra-backtest-bench`) and `BENCHMARKS.md`
  documenting ~1.7M bars/second on one core.

### Changed

- Execution depth: the portfolio is now signed (long **and** short positions),
  and stop-loss / take-profit are checked **intrabar** against each bar's OHLC and
  fill at the level (conservative stop-before-target ordering) instead of at the
  next close. Short entries/exits use `short_entry` / `short_exit`.

[Unreleased]: https://github.com/wickra-lib/wickra-backtest/commits/main
