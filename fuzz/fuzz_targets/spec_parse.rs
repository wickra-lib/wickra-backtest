#![no_main]
//! Fuzz the strategy-spec parser with arbitrary input.
//!
//! `StrategySpec::parse` must never panic: arbitrary text, malformed JSON,
//! valid JSON with the wrong shape, and undeclared indicator references all
//! have to surface as an `Err`, never a crash.

use libfuzzer_sys::fuzz_target;
use wickra_backtest_core::StrategySpec;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = StrategySpec::parse(text);
    }
});
