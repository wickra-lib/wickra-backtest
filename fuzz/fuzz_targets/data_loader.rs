#![no_main]
//! Fuzz the candle data loaders with arbitrary input.
//!
//! The CSV, JSON-Lines and JSON-array parsers must never panic: malformed
//! headers, non-numeric cells, truncated rows, arbitrary JSON and binary noise
//! all have to surface as an `Err`, never a crash.

use libfuzzer_sys::fuzz_target;
use wickra_backtest_data::{parse_csv, parse_json_array, parse_jsonl};

fuzz_target!(|data: &[u8]| {
    if let Ok(text) = std::str::from_utf8(data) {
        let _ = parse_csv(text);
        let _ = parse_jsonl(text);
        let _ = parse_json_array(text);
    }
});
