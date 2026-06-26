#![no_main]
//! Fuzz the unified `run_json` entry point with arbitrary input.
//!
//! `run_json` deserializes a whole request bundle (spec + candles + optional
//! feeds) and runs it. Any input — arbitrary bytes, malformed JSON, a valid
//! bundle with mismatched feed lengths or pathological candle values — must
//! surface as an `Err`, never a panic. This drives the full parse → validate →
//! engine path from a single untrusted string.

use libfuzzer_sys::fuzz_target;
use wickra_backtest_core::run_json;

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = run_json(text);
    }
});
