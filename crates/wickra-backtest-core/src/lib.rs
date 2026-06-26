//! # wickra-backtest-core
//!
//! Streaming-native, event-driven backtest engine built on the
//! [`wickra-core`](https://crates.io/crates/wickra-core) indicator kernels.
//!
//! The engine is **feed-agnostic**: it consumes a stream of bars and a
//! data-driven [`StrategySpec`], evaluates entry/exit rules over the exact same
//! O(1) indicator updates that power live Wickra, and produces a
//! [`BacktestReport`]. Because the indicator math is identical to live, and the
//! strategy is data (JSON) rather than code, a backtest and a live run over the
//! same spec produce identical signals — across all Wickra language bindings.
//!
//! This crate is the shared engine core; the historical-data driver lives in
//! `wickra-backtest` and live execution will live in a separate `wickra-bot`,
//! both depending on this core so "backtest == live" holds by construction.
//!
//! Status: **scaffold** (handoff-20, Phase 0). The public surface below is the
//! intended shape; modules are filled in over Phases 1–5.

#![forbid(unsafe_code)]

pub mod data;
pub mod engine;
pub mod error;
pub mod metrics;
pub mod portfolio;
pub mod registry;
pub mod report;
pub mod request;
pub mod rules;
pub mod spec;

pub use data::{
    Candle, CrossSection, CrossSectionMember, DerivativesTick, Level, OrderBook, TradePrint,
    TradeSide,
};
pub use engine::{
    run, run_with_capital, run_with_cross_section, run_with_deriv, run_with_orderbook,
    run_with_ref, run_with_trades, Feeds, StreamingBacktest, DEFAULT_CAPITAL,
};
pub use error::{BacktestError, Result};
pub use metrics::Metrics;
pub use portfolio::Trade;
pub use registry::EvalIndicator;
pub use report::{BacktestReport, EquityPoint, REPORT_SCHEMA_VERSION};
pub use request::{run_json, RunRequest};
pub use spec::{
    Condition, Costs, Execution, Feed, FillTiming, IndicatorSpec, IntPredicate, Operand,
    OperandExpr, OrderType, PriceField, Risk, Sizing, Slippage, StrategySpec, SPEC_VERSION,
};

/// The crate version, surfaced for diagnostics and binding parity checks.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// The JSON Schema for [`StrategySpec`], pretty-printed. Editors and tooling can
/// validate strategy specs against it; the committed
/// `schema/strategy_spec.schema.json` is generated from this.
#[must_use]
pub fn strategy_spec_schema() -> String {
    let schema = schemars::schema_for!(StrategySpec);
    serde_json::to_string_pretty(&schema).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_reported() {
        assert!(!version().is_empty());
    }

    #[test]
    fn strategy_spec_schema_is_committed() {
        // The committed schema must match what schemars generates. Regenerate with
        //   WICKRA_BLESS=1 cargo test -p wickra-backtest-core strategy_spec_schema
        let schema = strategy_spec_schema();
        assert!(schema.contains("StrategySpec"));
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../schema/strategy_spec.schema.json");
        if std::env::var("WICKRA_BLESS").is_ok() {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, format!("{schema}\n")).unwrap();
            return;
        }
        let committed =
            std::fs::read_to_string(&path).expect("schema file missing (run WICKRA_BLESS=1)");
        assert_eq!(schema, committed.trim_end(), "schema drift");
    }

    #[test]
    fn errors_render() {
        let e = BacktestError::UnknownIndicator("Foo".into());
        assert!(e.to_string().contains("Foo"));
    }
}
