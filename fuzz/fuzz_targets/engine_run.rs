#![no_main]
//! Fuzz the engine over arbitrary candle sequences.
//!
//! The fuzzer chunks the raw `f64` stream into `[open, high, low, close,
//! volume]` candles (with monotonic timestamps) and runs a fixed strategy over
//! them. Pathological values — NaN, ±inf, inverted high/low, zero volume,
//! extreme magnitudes — must surface as an `Err` from the candle validation,
//! never a panic. The strategy uses indicators and a price rule so the
//! indicator-update and rule-evaluation paths are exercised too.

use libfuzzer_sys::fuzz_target;
use wickra_backtest_core::{run_with_capital, Candle, StrategySpec};

fn candles_from(data: &[f64]) -> Vec<Candle> {
    data.chunks_exact(5)
        .enumerate()
        .map(|(i, ch)| Candle {
            time: i as i64,
            open: ch[0],
            high: ch[1],
            low: ch[2],
            close: ch[3],
            volume: ch[4],
        })
        .collect()
}

fuzz_target!(|data: Vec<f64>| {
    let candles = candles_from(&data);
    if candles.is_empty() {
        return;
    }
    let spec = StrategySpec::parse(
        r#"{"symbol":"x","timeframe":"1h",
            "indicators":{"ema":{"type":"Ema","params":[5]},"rsi":{"type":"Rsi","params":[14]}},
            "entry":{"all":[{"cross_above":[{"price":"close"},"ema"]},{"lt":["rsi",70]}]},
            "exit":{"cross_below":[{"price":"close"},"ema"]},
            "sizing":{"type":"fixed_fraction","fraction":0.5}}"#,
    )
    .expect("static spec parses");
    let _ = run_with_capital(&spec, &candles, 10_000.0);
});
