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

[Unreleased]: https://github.com/wickra-lib/wickra-backtest/commits/main
