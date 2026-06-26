#![no_main]
//! Fuzz the execution / fill model over arbitrary candle sequences.
//!
//! Same candle construction as `engine_run`, but the strategy stresses the
//! fill model specifically: a stop-loss, a take-profit and a trailing stop
//! (intrabar exits along the O→H→L→C path), a limit entry order, leverage and
//! both maker and taker fees with bps slippage. None of these may panic on any
//! candle stream; an invalid candle surfaces as an `Err`.

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
        r#"{"symbol":"x","timeframe":"1h","indicators":{},
            "entry":{"gt":[{"price":"close"},0]},
            "exit":{"lt":[{"price":"close"},0]},
            "sizing":{"type":"fixed_fraction","fraction":0.5},
            "costs":{"maker_bps":2,"taker_bps":5,"slippage":{"type":"fixed_bps","bps":3}},
            "risk":{"stop_loss_pct":2.0,"take_profit_pct":5.0,"trailing_stop_pct":3.0,"max_leverage":3.0},
            "execution":{"order_type":"limit","limit_offset_pct":-0.5}}"#,
    )
    .expect("static spec parses");
    let _ = run_with_capital(&spec, &candles, 10_000.0);
});
