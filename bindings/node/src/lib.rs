//! Node.js bindings for `wickra-backtest`.
//!
//! Exposes `run(...)` which takes parallel OHLCV arrays and a JSON strategy
//! spec and returns the `BacktestReport` as a JSON string (the caller
//! `JSON.parse`s it). Time is taken as `number[]` (epoch) and truncated to an
//! integer internally.

#![allow(clippy::needless_pass_by_value)]

use napi::bindgen_prelude::*;
use napi_derive::napi;

use wickra_backtest_core::{
    run_json as core_run_json, run_with_capital, Candle, StrategySpec, DEFAULT_CAPITAL,
};

fn to_napi<E: std::fmt::Display>(e: E) -> Error {
    Error::from_reason(e.to_string())
}

/// Run a backtest. Returns the report as a JSON string.
#[napi]
#[allow(clippy::too_many_arguments)]
pub fn run(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    time: Vec<f64>,
    spec_json: String,
    capital: Option<f64>,
) -> Result<String> {
    let n = open.len();
    for (name, len) in [
        ("high", high.len()),
        ("low", low.len()),
        ("close", close.len()),
        ("volume", volume.len()),
        ("time", time.len()),
    ] {
        if len != n {
            return Err(Error::from_reason(format!(
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

    let spec = StrategySpec::parse(&spec_json).map_err(to_napi)?;
    let report =
        run_with_capital(&spec, &candles, capital.unwrap_or(DEFAULT_CAPITAL)).map_err(to_napi)?;
    serde_json::to_string(&report).map_err(to_napi)
}

/// Run a backtest from a single JSON request bundling candles, spec and optional
/// feeds. Returns the report as a JSON string.
#[napi]
pub fn run_json(request_json: String) -> Result<String> {
    core_run_json(&request_json).map_err(to_napi)
}

/// The crate version.
#[napi]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
