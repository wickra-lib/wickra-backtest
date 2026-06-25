<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514" alt="Wickra — streaming-first technical indicators" width="100%"></a>
</p>

[![Built on Wickra](https://img.shields.io/badge/built%20on-wickra-3b82f6)](https://github.com/wickra-lib/wickra)
[![Status](https://img.shields.io/badge/status-alpha%20(WIP)-orange)](https://github.com/wickra-lib/wickra-backtest)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/license.svg)](https://github.com/wickra-lib/wickra#license)

# Wickra — Backtest

**Backtest and live — byte-identical, in 10 languages.** A streaming-native,
event-driven backtester built on the [Wickra](https://github.com/wickra-lib/wickra)
indicator core.

The engine consumes the **exact same `wickra-core` O(1) indicator kernels** that
power live Wickra, and a strategy is **data (a JSON spec), not code** — so a
backtest and a live run over the same spec produce identical signals, across
every Wickra language binding. The same engine, fed live instead of historical
bars, becomes the live bot: **backtest ≡ live, by construction.**

Why it is different from vectorbt / backtrader:

- **O(1) per tick** — years of tick data in seconds, not hours (no recompute-on-every-tick).
- **Backtest = live, value-identical across 10 languages** — no reimplementation drift.
- **Microstructure** — backtest strategies on order-book imbalance, footprint, funding and open interest, not just OHLCV.
- **Polyglot** — the same `StrategySpec` runs from Rust, Python, Node, the browser (WASM), Go, C#, Java, C/C++ and R.

## Status

**Alpha / work in progress** — Phase 0 scaffold (see `handoff-20`). The workspace
compiles; the engine, strategy spec, execution model and bindings land over the
following phases. Not yet released to any registry.

## Workspace

| Crate | Role |
|-------|------|
| `wickra-backtest-core` | feed-agnostic engine: strategy spec, rules, sizing, execution, portfolio, report |
| `wickra-backtest-data` | data loaders (CSV / Parquet / JSONL) |
| `wickra-backtest` | facade: re-exports the engine + the historical backtest runner |

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.

> Not a trading system. Backtest results are not indicative of future performance. Use at your own risk.
