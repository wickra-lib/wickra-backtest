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

// A glob, not a name list. A hand-kept list is a list that goes stale: this one
// had drifted to the point where `BacktestReport` was exported and `Metrics` --
// the type of its own `.metrics` field -- was not, and `StreamingBacktest` was
// exported while `Feeds`, which `step_with_feeds` takes, was not. Callers could
// hold those values but not name their types. The explicit `core` alias above
// still wins over this glob for the `data` module, which is Rust's precedence
// rule for glob imports.
pub use wickra_backtest_core::*;

/// The crate version, surfaced for diagnostics.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    //! The facade's job is to make the core's surface reachable under one name.
    //! These name the types that were missing from the old hand-kept list; if the
    //! glob is ever narrowed back to a list, this stops compiling.

    use super::*;

    #[test]
    fn version_is_the_crate_version() {
        assert_eq!(version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn a_report_and_the_type_of_its_metrics_field_are_both_reachable() {
        fn takes(_: &BacktestReport, _: &Metrics) {}
        let _ = takes;
        let _: u32 = REPORT_SCHEMA_VERSION;
    }

    #[test]
    fn the_streaming_handle_and_the_feeds_it_steps_with_are_both_reachable() {
        fn steps(handle: &mut StreamingBacktest<'_>, candle: &Candle, feeds: &Feeds) -> Result<()> {
            handle.step_with_feeds(candle, feeds)
        }
        let _ = steps;
    }

    #[test]
    fn the_json_entry_point_and_its_request_type_are_reachable() {
        fn takes(_: RunRequest) -> fn(&str) -> Result<String> {
            run_json
        }
        let _ = takes;
    }

    #[test]
    fn a_spec_can_be_built_from_typed_parts_not_only_parsed_from_json() {
        // Every type named here comes from `spec`; none was re-exported before, so
        // a caller could only reach a StrategySpec by parsing JSON.
        let _: Option<Condition> = None;
        let _: Option<Costs> = None;
        let _: Option<Execution> = None;
        let _: Option<Feed> = None;
        let _: Option<FillTiming> = None;
        let _: Option<IndicatorSpec> = None;
        let _: Option<IntPredicate> = None;
        let _: Option<Operand> = None;
        let _: Option<OperandExpr> = None;
        let _: Option<OrderType> = None;
        let _: Option<PriceField> = None;
        let _: Option<Risk> = None;
        let _: Option<Sizing> = None;
        let _: Option<Slippage> = None;
        let _: u32 = SPEC_VERSION;
    }

    #[test]
    fn the_data_types_the_engine_consumes_are_reachable() {
        let _: Option<CrossSection> = None;
        let _: Option<CrossSectionMember> = None;
        let _: Option<Level> = None;
        let _: Option<TradeSide> = None;
        let _: Option<DerivativesTick> = None;
        let _: Option<OrderBook> = None;
        let _: Option<TradePrint> = None;
        let _: Option<Trade> = None;
        let _: f64 = DEFAULT_CAPITAL;
    }
}
