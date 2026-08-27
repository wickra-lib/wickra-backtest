# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and this project adheres
to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `StreamingBacktest::new_owned` — a streaming backtest that owns its spec, so the
  handle carries no borrow and can be held across `step`s indefinitely (the
  borrowing `new` is unchanged).
- WASM: a `StreamingBacktest` handle (`new` / `step` / `stepWithRef` / `equity` /
  `latestEquity` / `numTrades` / `finish`) that drives the engine bar-by-bar in
  the browser — the same kernel and the same values as the batch `run`.
- Workspace scaffold: `wickra-backtest-core` (engine core), `wickra-backtest-data`
  (loaders) and the `wickra-backtest` facade, depending on `wickra-core`.
- `BacktestError` error type and crate `version()` helpers.
- `Candle` input type with conversion to `wickra-core` and derived prices.
- Microstructure feed input types (`TradePrint`, `OrderBook` + `Level`,
  `DerivativesTick`, `TradeSide`): serde-friendly, validated value types with
  `to_core()` conversions into the `wickra-core` trade / order-book / derivatives
  inputs — the data foundation for the trade-flow, order-book and derivatives
  indicators.
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
- WASM binding (`wickra-backtest-wasm`, wasm-bindgen): `run(...)` over
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
- Documentation: a strategy spec reference (`docs/STRATEGY_SPEC.md`) covering the
  full DSL — operands, conditions, sizing, costs, slippage, risk and execution —
  and a cookbook (`docs/COOKBOOK.md`) of six ready-to-run strategies (RSI mean
  reversion, MACD trend long/short, Bollinger breakout, Donchian breakout,
  funding carry and order-book imbalance). The strategies live under
  `examples/strategies/` and an `example_specs` test parses and validates every
  one, so the documented examples can never drift from the engine.
- Property tests (`tests/properties.rs`, proptest): the parser and engine never
  panic on any input — `StrategySpec::parse` and `run_json` always return a
  `Result` for arbitrary text, the engine tolerates arbitrary candles (NaN, ±inf,
  inverted high/low), and the data loaders tolerate arbitrary text. This guards
  the promise that no strategy input can crash the backtester.
- cargo-fuzz targets (`fuzz/`, nightly): `spec_parse`, `run_json`, `engine_run`,
  `fill_model` and `data_loader` libfuzzer harnesses for the parse and execution
  entry points. They mirror the property tests above, which prove the same
  never-panic invariants on the stable toolchain.
- Alternative-chart bar transforms (`to_renko`, `to_kagi`, `to_pnf`): rebuild a
  candle stream into Renko bricks, Kagi segments or Point-and-Figure columns
  using `wickra-core`'s `BarBuilder` types (so the bars match live Wickra), each
  emitted as a synthetic candle with the bar's edge prices and a sequential
  timestamp. The CLI gains `--renko BOX`, `--kagi REVERSAL` and `--pnf
  BOX:REVERSAL`, mutually exclusive with each other and the resample options, so
  `wkbt run` can backtest a strategy on price-driven bars instead of time bars.
- Binance importer: `parse_binance_klines` (always available, parses the
  Binance `klines` REST response — OHLCV-as-strings, millisecond open times) and
  `fetch_klines(symbol, interval, limit)` behind the `binance` feature, which
  fetches candles over HTTP. The `wkbt fetch` subcommand (also behind `binance`)
  writes them to CSV / JSON / JSON Lines. The parser is unit-tested offline; the
  fetch was verified end-to-end against the live API.
- Streaming entry point `run_stream` and the `wkbt --stream` mode: drive the
  engine one bar at a time and emit the equity curve incrementally (the live
  path), with a report byte-identical to the batch runner.
- Apache Parquet loading behind the `parquet` feature (`load_parquet`, dispatched
  for `.parquet` files): reads `time, open, high, low, close[, volume]` columns
  (matched case-insensitively, integer or floating-point). The CLI gains a
  matching `parquet` feature so `wkbt run --data history.parquet` works. The
  arrow / parquet stack is heavy, so it is opt-in rather than always compiled.
- Resampling (`wickra-backtest-data`): `resample_by_count` aggregates fixed groups
  of bars (e.g. five 1-minute into one 5-minute) and `resample_by_interval`
  buckets by a timestamp interval (`floor(time / interval)`). Each bucket opens at
  the first open, closes at the last close, takes the extreme high/low and sums
  volume.
- `wkbt` command-line backtester (`wickra-backtest-cli`): `wkbt run --data … --spec …
  [--capital N] [--resample-count N | --resample-interval I] [--report …]
  [--trades …] [--equity …]` prints a metrics summary and optionally writes the
  report (JSON) and trade/equity streams (JSON Lines); `--resample-*` aggregates
  the data to a coarser timeframe first. `wkbt schema` prints the strategy-spec
  JSON Schema.
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
- JSON Schema for the strategy spec: `strategy_spec_schema()` emits a draft-07
  schema (generated from the spec types with schemars), committed as
  `schema/strategy_spec.schema.json` for editor/tooling validation. A test pins
  the committed file to the generated schema.
- Unified JSON entry point (`run_json` / `RunRequest`): a single JSON document
  bundles the spec, candles, capital and any optional per-bar feeds (reference,
  derivatives, order book, trades, cross-section); `run_json` deserializes it,
  threads the present feeds through the engine and returns the report JSON. This
  is the uniform surface the language bindings wrap, so every binding can run any
  feed combination by passing one JSON string. Each feed must match the candle
  count. The entry point is now exposed through every binding — Python
  `run_json`, Node.js `runJson`, WASM `run_json`, C `wickra_backtest_run_json`,
  C# `RunJson`, Go `RunJSON`, Java `runJson` and R `backtest_run_json` — each
  with a round-trip test, so any feed combination is reachable from all ten
  languages by passing one request document instead of marshalling variable
  per-bar feed arrays across the FFI.
- Sortino and Calmar ratios in the metrics: Sortino is the per-bar mean return
  over the downside deviation (only returns below zero are penalised), and
  Calmar is the total return divided by the maximum drawdown. Both complete the
  documented report schema alongside Sharpe.
- Streaming API (`StreamingBacktest`): feed bars one at a time with `step` and
  finalize with `finish`. The historical `run` / `run_with_capital` is exactly
  this fed from a slice, so backtest and live share one code path — pointing
  `step` at a live feed turns the same engine into the live bot. A test asserts
  the streamed report is identical to the batch report.
- Golden parity corpus (`golden/`): shared strategy cases (price threshold, EMA
  crossover, RSI mean reversion, MACD long/short) and the canonical report each
  must produce. The Rust integration test (`tests/golden.rs`) is the anchor
  (`WICKRA_BLESS=1` regenerates the expected reports); the Go, Node.js and
  Python bindings assert their output against the same reports — byte-for-byte
  for the JSON-returning bindings, value-for-value for Python. This pins
  cross-language equality and is regression-proof. **All ten language reaches**
  are wired in and verified against the same reports: Rust, Go, Node.js, Python,
  C and C++ (through the C ABI), C#, Java, WASM and R.
- Feed golden parity corpus (`golden/requests/`, `golden/expected_json/`): five
  request bundles that each drive a microstructure feed path — derivatives
  funding, top-of-book imbalance, cumulative volume delta, advance-decline
  breadth and a pairwise correlation reference series — through the unified
  `run_json` entry point. The Rust integration test (`tests/golden_json.rs`) is
  the anchor (`WICKRA_BLESS=1` regenerates the expected reports), and all ten
  language reaches assert their own `run_json` output against the same reports.
  This pins cross-language equality for the feed paths, not just the plain OHLCV
  path — the microstructure differentiator is now regression-proof end to end.

### Changed

- Intrabar stop / target fills are now gap-aware: a stop still fills at its level
  when price trades through it intrabar, but if the bar *opens* beyond the level
  (a gap), the fill is the open — the worse price for a stop or trailing stop,
  the better price for a take-profit — instead of the unreachable level the bar
  never traded at. This makes gapped exits conservative rather than optimistic; a
  long stop fills at `min(level, open)`, a long target at `max(level, open)`, and
  a short is the mirror. The golden corpus is unchanged (its cases do not gap
  through a level).
- Spread and volume-impact slippage: the `spread` model moves the fill by the
  order book's half-spread relative to the mid (needs an order-book feed), and
  `volume_impact` by `coef * order_qty / bar_volume`. Both are now applied
  per-fill (slippage is computed at fill time from the bar's feeds instead of a
  fixed scalar), completing the three slippage models. `fixed_bps` is unchanged.
- Maker / taker fees: a resting limit-order fill now pays `costs.maker_bps`
  (it provides liquidity), while market and stop fills pay `costs.taker_bps`.
  Previously `maker_bps` was unused and every fill paid the taker fee.
- Liquidation: with `risk.liquidation` set, a leveraged position is force-closed
  intrabar at its bankruptcy price (where account equity reaches zero,
  `-cash / qty`) — checked against the bar's adverse extreme, after stop-loss /
  take-profit. Only bites above 1x leverage; the trade's reason is
  `"liquidation"`. Completes the perpetual trio (leverage + funding +
  liquidation).
- Perpetual funding cost: `costs.funding` charges funding to an open position
  each bar from the derivatives feed (`payment = qty * mark_price * funding_rate`
  — longs pay when the rate is positive, shorts receive), reflected in `cash`,
  equity and `fees_paid`. Requires a derivatives feed; default off.
- Cross-section (market-breadth) feed: the registry now includes the 15
  cross-section (`Input = CrossSection`) breadth indicators (advance/decline and
  ratio, McClellan oscillator / summation, TRIN, breadth thrust, new highs/lows,
  high-low index, up/down volume, …), fed a per-bar market panel of
  `CrossSectionMember`s. `run_with_cross_section(spec, candles, sections,
  capital)` drives the feed; `CrossSection` / `CrossSectionMember` are new data
  types. The `above_ma` / `on_buy_signal` member flags are not settable through
  wickra-core's non-exhaustive `Member`, so the two indicators reading them
  (`PercentAboveMa`, `BullishPercentIndex`) see them as `false`. Registry: 480 →
  **495** — the complete backtestable wickra-core catalog (the remaining types
  are bar-builders and profile outputs, not single-value indicators).
- Trade-quote feed: the registry now includes the 3 trade-quote
  (`Input = TradeQuote`) indicators (effective spread, realized spread, Kyle's
  lambda), fed each bar trade paired with the prevailing mid (the order book's
  mid if an order-book feed is present, else the bar close). Registry: 477 →
  **480** — every single-instrument, pairwise and microstructure-feed indicator
  is now registered (only the multi-asset cross-section family remains).
- Trade feed: the registry now includes the 8 trade-flow (`Input = Trade`)
  indicators (cumulative volume delta, trade imbalance, signed volume, VPIN,
  Amihud illiquidity, Roll measure, PIN, trade-sign autocorrelation), fed the
  bar's trades. The `TradeIn` wrapper replays each bar's trades in order and
  returns the value after the last; `run_with_trades(spec, candles, trades,
  capital)` takes one trade list per bar. Registry: 469 → **477**.
- Order-book feed: the registry now includes the 7 order-book
  (`Input = OrderBook`) indicators (top-of-book / full-depth imbalance,
  microprice, quoted spread, depth slope, order-flow imbalance), fed the bar's
  order-book snapshot. The per-bar feeds are now bundled in a `Feeds` struct;
  `run_with_orderbook` (and `StreamingBacktest::step_with_feeds(candle, &Feeds)`)
  drive a backtest with an order-book feed. Registry: 462 → **469**.
- Derivatives feed: the registry now includes the 17 derivatives
  (`Input = DerivativesTick`) indicators (funding rate, open-interest delta /
  momentum, long-short ratio, taker buy/sell ratio, perpetual premium, …), fed
  the bar's derivatives tick. `EvalIndicator::update` now takes a `BarInput`
  context (candle + optional reference + optional derivatives tick) instead of
  loose arguments; `run_with_deriv` (and `StreamingBacktest::step_with_feeds`)
  drive a backtest with a per-bar derivatives feed. Without a feed the
  derivatives indicators yield nothing. Registry: 445 → **462**.
- Pairwise indicators: the registry now includes all 24 pairwise
  (`Input = (f64, f64)`) indicators — 19 scalar-output (correlation, beta, spread
  z-score, variance ratio, …) and 5 multi-output (cointegration, Kalman hedge
  ratio, lead-lag cross-correlation, relative strength, spread Bollinger bands)
  whose fields are addressed as `"name.field"` — all fed `(close,
  reference_close)`. `run_with_ref` (and `StreamingBacktest::step_with_ref`)
  supply a reference price series whose per-bar close is the second input;
  `EvalIndicator::update` gained a `reference: Option<f64>` argument that
  single-instrument indicators ignore. Without a reference a pairwise indicator
  yields nothing. Registry: 421 → **445**.
- Close-to-close execution: `execution.fill_timing` of `"close"` fills market
  orders on the signalling bar's own close (same bar) instead of the next bar's
  open. It is an opt-in, deliberately optimistic mode — the fill uses the very
  close that produced the signal, which is not tradeable live — so the default
  stays `next_open` (look-ahead-free). Validation rejects it together with
  limit/stop orders or `latency_bars` (both next-bar concepts).
- Volume-participation partial fills: with `execution.partial_fills` and
  `execution.max_participation`, an entry fills at most `max_participation *
  bar_volume` (immediate-or-cancel — the unfilled remainder is cancelled), so a
  strategy can't assume unlimited liquidity. Validation requires
  `max_participation` when `partial_fills` is set; the default is off.
- Execution latency: `execution.latency_bars` delays every order by that many
  bars before it becomes eligible to fill (on top of the look-ahead-free
  next-bar fill). A limit/stop order only starts checking its level once the
  latency has elapsed. The default is 0 (fill at the next bar).
- Limit and stop entry orders: `execution.order_type` of `"limit"` / `"stop"`
  rests an order at a percent offset from the signal bar's close
  (`limit_offset_pct` / `stop_offset_pct`). A limit fills when a later bar trades
  through it (at the level, or the open if it gaps past); a stop fills on the
  breakout; otherwise the order keeps working (good-till-filled) without being
  re-decided. A limit/stop without its offset, or `"stop_limit"`, is rejected by
  validation. Market orders (the default) are unchanged.
- Leverage and position sizing: `risk_per_trade` sizing is now implemented
  (quantity sized so a stop-loss hit loses `risk_pct` of equity, requires
  `risk.stop_loss_pct`), and every order's notional is capped by `max_leverage`
  and `max_position_pct`. **Without `max_leverage` the cap is 1x equity — no
  leverage by default** — so `fixed_cash` / `fixed_qty` orders can never exceed
  what the account funds; set `max_leverage` to trade on margin.
- `vol_target` sizing is now implemented: the position notional is scaled so the
  position's per-bar return volatility approximates `target_vol`, from the
  realized volatility of close-to-close returns over `lookback` bars
  (`notional = equity * target_vol / realized_vol`, then capped by the leverage
  limits). No position is taken until `lookback` bars of history exist — all five
  sizing models are now supported.
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

### Fixed

- **The workspace defined no build profiles at all**, so release artifacts were
  built without LTO and — more seriously — the panic strategy for four cdylibs was
  whatever the default happened to be. Cargo refuses `panic` in a per-package
  profile, so the workspace `[profile.release]` is the only place that setting can
  live: the Python, Node, WASM and C ABI libraries are all built with it. The C ABI
  wraps every entry point in `catch_unwind` and pyo3 and napi turn a caught panic
  into a language-level exception; under `abort` none of that can run and a panic
  reaching an FFI boundary would take the host process down instead of raising.
  `panic = "unwind"` is now stated with that reasoning rather than inherited by
  luck, alongside `lto = "fat"`, `codegen-units = 1` and `strip`, plus the bench
  and dev profiles.

- **The workspace had no `[workspace.lints.rust]` block**, so `unsafe_code`,
  `unused_must_use`, `unreachable_pub` and `missing_debug_implementations` were
  unenforced across all nine crates while the clippy set beside them was strict.
  The four lints are now set, with `unsafe_code = "forbid"`. The two crates that
  cannot live under a forbid opt out and restate the same set locally rather than
  losing every lint with it: the C ABI, which is the FFI boundary and needs
  `unsafe`, and the Node binding, whose `#[napi]` macro expands to
  `#[allow(unsafe_code)]` — a forbid cannot be lifted, so that one uses `deny`,
  which the macro can override while hand-written unsafe still has to be argued
  for. Turning the block on immediately found seven types with no `Debug`
  implementation, including `StreamingBacktest` itself; six are derives, and the
  streaming handle has a hand-written one because it holds
  `Box<dyn EvalIndicator>`, which no derive can reach.

- **Nothing held the ten language reaches to the C ABI.** Each binding has its own
  suite and the golden corpus compares *values*, so a binding that never grew a
  method has no test to fail — the WASM streaming handle, which no other language
  can reach, sat there unreported. `scripts/check_binding_surface.py` now derives
  the contract from `bindings/c/include/wickra_backtest.h` and checks each
  language's public surface for it, spelled the way that language spells it
  (`run_json` → Node's `runJson`, Go's `RunJSON`, R's `backtest_run_json`), and
  runs as a `binding-surface` CI job. It matches *declarations*, not occurrences:
  a first version searched the raw text and let a renamed Go export pass because
  its own doc comment still named it. A binding that is ahead of the ABI is
  reported as a note rather than a failure, which is how the WASM streaming gap
  now shows up on every run until it is closed.

- **The fuzz job ran on a rolling `nightly`.** A smoke test whose toolchain
  changes underneath it fails on days that have nothing to do with this code, and
  a job that fails at random stops being read. It is pinned to the same dated
  nightly the indicator repository uses, to be bumped deliberately. The
  `cargo-fuzz` install gained the fail-fast timeout it was missing, and a
  build-every-target step now precedes the five runs: `fuzz run <target>` compiles
  only the target it runs, so a sixth target added later and forgotten in the run
  list would never be built against the core API at all.

- **Four of the five Python versions this project ships were never tested.**
  `pyproject.toml` declares `requires-python = ">=3.9"` and `release.yml` builds
  abi3-py39 wheels, so a single wheel serves 3.9 through 3.13 — while CI ran the
  binding on 3.12 alone. The matrix is now three operating systems by
  `3.9 / 3.11 / 3.12 / 3.13`, twelve checks instead of three, matching the
  indicator repository. An abi3 wheel is exactly the case where testing one
  interpreter proves the least: the whole point is that the same binary is loaded
  by all of them.

- **`ci.yml` carried none of the hardening the rest of the repository already
  had.** The workflow had no `env:` block at all, while this repo's own
  `release.yml` carries the network-retry block verbatim and `bench.yml` carries
  half of it — so the one workflow running 15 jobs across three operating systems,
  the one most exposed to a registry or CDN blip, was the only one left bare. It
  now has the same block as the indicator repo, including `RUSTFLAGS: "-D
  warnings"` (verified: the workspace builds and tests clean under it). Every job
  gained the 30-minute backstop that caps a wedged run instead of leaving it to
  GitHub's six-hour default, all 14 cache restores are now `continue-on-error`
  with a 6-minute cap because a cache is an optimisation and must never block a
  job, and `setup-python` and `setup-node` are wrapped in the wait-and-retry the
  Windows CDN flake requires.

- **Every `rust-cache` pin named a version it does not point at.** All 19 uses
  across `ci.yml`, `release.yml` and `bench.yml` pinned `e18b497` with the comment
  `# v2`. That commit carries no tag at all — it is an untagged master commit, 45
  behind the `v2` tip — so the comment was never true for it, and because
  Dependabot cannot map an untagged SHA to a version it would never have opened a
  bump either. The pin is now `6323deb`, commented with the exact release it is,
  `# v2.9.2`, matching the indicator repository. Every other pinned action was
  checked the same way and all of them resolve to the commit their comment names;
  the one remaining floating comment, `lychee-action # v2`, is now `# v2.9.0`. A
  floating major is accurate only until the tag moves, and then it silently
  becomes a false claim a reviewer has no way to spot.

[Unreleased]: https://github.com/wickra-lib/wickra-backtest/commits/main
