//! WebAssembly bindings for `wickra-backtest` — run a backtest in the browser.
//!
//! `run(...)` takes parallel `Float64Array`s and a JSON strategy spec and
//! returns the `BacktestReport` as a JSON string (the caller `JSON.parse`s it).
//! Pairs with `live.wickra.org`: backtest a strategy entirely client-side, with
//! the same kernel and the same values as the Rust, Python and Node bindings.

use wasm_bindgen::prelude::*;

use wickra_backtest_core::{run_with_capital, Candle, StrategySpec};

fn to_js<E: std::fmt::Display>(e: E) -> JsError {
    JsError::new(&e.to_string())
}

/// Run a backtest. Returns the report as a JSON string.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn run(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    volume: &[f64],
    time: &[f64],
    spec_json: &str,
    capital: f64,
) -> Result<String, JsError> {
    let n = open.len();
    for (name, len) in [
        ("high", high.len()),
        ("low", low.len()),
        ("close", close.len()),
        ("volume", volume.len()),
        ("time", time.len()),
    ] {
        if len != n {
            return Err(JsError::new(&format!(
                "{name} length {len} does not match open length {n}"
            )));
        }
    }

    let candles: Vec<Candle> = (0..n)
        .map(|i| Candle {
            time: time[i] as i64,
            open: open[i],
            high: high[i],
            low: low[i],
            close: close[i],
            volume: volume[i],
        })
        .collect();

    let spec = StrategySpec::parse(spec_json).map_err(to_js)?;
    let report = run_with_capital(&spec, &candles, capital).map_err(to_js)?;
    serde_json::to_string(&report).map_err(to_js)
}

/// The crate version.
#[wasm_bindgen]
#[must_use]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
