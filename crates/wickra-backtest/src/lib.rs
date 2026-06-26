//! # wickra-backtest
//!
//! Streaming-native, event-driven backtester for the
//! [Wickra](https://github.com/wickra-lib/wickra) technical-indicator library.
//!
//! This facade re-exports the engine ([`wickra-backtest-core`]) and the data
//! loaders ([`wickra-backtest-data`]) behind one crate, plus the historical
//! backtest runner and reports.
//!
//! The same engine, fed live instead of historical bars, becomes the live bot —
//! so **backtest == live, byte-identical**, and (because the strategy is a JSON
//! spec, not code) identical across every Wickra language binding.
//!
//! Status: **scaffold** (handoff-20, Phase 0).

#![forbid(unsafe_code)]

pub use wickra_backtest_core as core;
pub use wickra_backtest_data as data;

pub use wickra_backtest_core::{
    run, run_with_capital, strategy_spec_schema, BacktestError, BacktestReport, Candle, Result,
    StrategySpec, StreamingBacktest,
};

/// The crate version, surfaced for diagnostics.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
