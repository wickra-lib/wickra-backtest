# Roadmap

`wickra-backtest` is pre-1.0. The spec format and report schema may still change
(they carry `spec_version` / `schema_version` fields for forward compatibility).

## Done

- Event-driven streaming engine; backtest and live share one code path.
- Full execution model: maker/taker fees, three slippage models, perpetual
  funding, liquidation, leverage and position caps, all order types, simulated
  latency, partial fills, and intrabar stop-loss / take-profit / trailing stop.
- Five position-sizing models and the complete rule DSL.
- 495 indicators, generated from the `wickra-core` sources, across the scalar,
  candle, multi-output, pairwise, derivatives, order-book, trade, trade-quote
  and cross-section families.
- Data loaders: CSV, JSON, JSON Lines and Apache Parquet; resampling and
  Renko / Kagi / Point-and-Figure bar transforms.
- Metrics: PnL, return, Sharpe, Sortino, Calmar, max drawdown, win rate,
  profit factor.
- Ten language bindings (Rust, Python, Node.js, WASM, C, C++, C#, Go, Java, R),
  all byte-identical, pinned by a golden corpus covering the OHLCV path and all
  four microstructure feed families.
- `wkbt` CLI and a unified `run_json` request entry point in every binding.

## Planned

- **Multi-asset / portfolio backtests** — run a strategy across a symbol panel
  with shared capital and cross-sectional ranking.
- **Exchange data importers** — pull historical candles directly (e.g. Binance)
  into the loader.
- **A live tick / streaming source** wired through `StreamingBacktest`.
- **An honest comparison benchmark** against vectorbt and backtrader.
- **A dedicated microstructure strategy guide.**
- **Nightly fuzz targets** (`spec_parse`, `engine_run`, `fill_model`,
  `data_loader`) to complement the stable-toolchain property tests.

## Toward 1.0

1.0 ships when the spec format and report schema are stable and the public CI,
coverage and supply-chain gates are green across all ten languages.

Suggestions and use cases are welcome — see [SUPPORT.md](SUPPORT.md).
