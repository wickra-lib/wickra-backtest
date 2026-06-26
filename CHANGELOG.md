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
- Node.js binding (`wickra-backtest`, napi-rs): `run(open, high, low, close, volume, time, specJson, capital?)`
  returns the report as a JSON string. The same strategy is byte-identical to the
  Rust and Python results.
- WebAssembly binding (`wickra-backtest-wasm`, wasm-bindgen): `run(...)` over
  `Float64Array`s returns the report JSON — **backtest in the browser**, with the
  same kernel and values as the Rust/Python/Node bindings (four-language parity).
- C ABI (`wickra-backtest-c`): a cdylib/staticlib exposing
  `wickra_backtest_run(...) -> int` (report written to `*out_json`, freed with
  `wickra_backtest_free_string`), `wickra_backtest_version()`, with a
  cbindgen-generated `wickra_backtest.h` and a C example that also compiles as
  C++ (the header is `extern "C"`). This is the hub for the Go / C# / Java / R
  bindings. No panic crosses the boundary; the FFI round-trip is byte-identical
  to the other bindings — this is the C and C++ language reach, giving
  ten-language parity (Rust, Python, Node.js, WASM, C, C++, C#, Go, Java, R).
- C# binding (`Wickra.Backtest`, P/Invoke over the C ABI): `Backtester.Run(open,
  high, low, close, …, spec, capital)` returns the report JSON. xUnit round-trip
  test is byte-identical to the other bindings (five-language parity).
- Java binding (`org.wickra:wickra-backtest`, Foreign Function and Memory API over
  the C ABI): `Backtester.run(open, high, low, close, …, spec, capital)` returns
  the report JSON. JUnit round-trip test is byte-identical to the other bindings
  (six-language parity). FFM is stable since Java 22 — no preview flags.
- Go binding (`github.com/wickra-lib/wickra-backtest-go`, cgo over the C ABI):
  `wickrabacktest.Run(open, high, low, close, …, spec, capital)` returns the
  report JSON. The `go test` round-trip is byte-identical to the other bindings
  (seven-language parity); cgo links directly against the shared library.
- R binding (`wickrabacktest`, compiled C glue calling the C ABI via `.Call`):
  `backtest_run(open, high, low, close, …, spec, capital)` returns the report
  JSON. A base-R round-trip test is byte-identical to the other bindings
  (eight-language parity). The header and library paths are supplied through the
  `WKBT_INC` / `WKBT_LIB` environment variables.
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
- Golden parity corpus (`golden/`): shared strategy cases (price threshold, EMA
  crossover, RSI mean reversion, MACD long/short) and the canonical report each
  must produce. The Rust integration test (`tests/golden.rs`) is the anchor
  (`WICKRA_BLESS=1` regenerates the expected reports); the Go, Node.js and
  Python bindings assert their output against the same reports — byte-for-byte
  for the JSON-returning bindings, value-for-value for Python. This pins
  cross-language equality and is regression-proof. **All ten language reaches**
  are wired in and verified against the same reports: Rust, Go, Node.js, Python,
  C and C++ (through the C ABI), C#, Java, WASM and R.

### Changed

- Leverage and position sizing: `risk_per_trade` sizing is now implemented
  (quantity sized so a stop-loss hit loses `risk_pct` of equity, requires
  `risk.stop_loss_pct`), and every order's notional is capped by `max_leverage`
  and `max_position_pct`. **Without `max_leverage` the cap is 1x equity — no
  leverage by default** — so `fixed_cash` / `fixed_qty` orders can never exceed
  what the account funds; set `max_leverage` to trade on margin. (`vol_target`
  sizing remains unimplemented and still errors.)
- Registry expanded from a curated ~26 indicators to **421**, generated by
  `tools/gen_registry.py` directly from the wickra-core indicator sources (the
  `Indicator` impls, `new` signatures and Output structs) joined with the golden
  manifests for default parameters: **336 scalar** (`Input = f64` fed the close,
  or `Input = Candle`) and **85 multi-output** indicators whose named fields are
  referenced in the spec as `"name.field"`, plus friendly aliases (`Macd`,
  `Bollinger`). Pairwise `(f64, f64)`, cross-section, derivatives, trade,
  order-book and quote inputs are structurally out of scope for a
  single-instrument bar backtester and are not registered. `registry.rs` is now
  a generated file; a build-all test constructs every indicator with valid
  defaults, and an engine test drives a backtest with a generated indicator
  (`Alma`) that was never in the original registry.
- Execution depth: the portfolio is now signed (long **and** short positions),
  and stop-loss / take-profit are checked **intrabar** against each bar's OHLC and
  fill at the level (conservative stop-before-target ordering) instead of at the
  next close. Short entries/exits use `short_entry` / `short_exit`.

[Unreleased]: https://github.com/wickra-lib/wickra-backtest/commits/main
