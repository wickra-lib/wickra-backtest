//! WebAssembly bindings for `wickra-backtest` — run a backtest in the browser.
//!
//! `run(...)` takes parallel `Float64Array`s and a JSON strategy spec and
//! returns the `BacktestReport` as a JSON string (the caller `JSON.parse`s it).
//! Pairs with `live.wickra.org`: backtest a strategy entirely client-side, with
//! the same kernel and the same values as the Rust, Python and Node bindings.

use wasm_bindgen::prelude::*;

use wickra_backtest_core::{
    run_json as core_run_json, run_with_capital, Candle, StrategySpec,
    StreamingBacktest as CoreStreaming,
};

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

/// Run a backtest from a single JSON request bundling candles, spec and optional
/// feeds. Returns the report as a JSON string.
#[wasm_bindgen]
pub fn run_json(request_json: &str) -> Result<String, JsError> {
    core_run_json(request_json).map_err(to_js)
}

/// The crate version.
#[wasm_bindgen]
#[must_use]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// A streaming backtest handle: build it from a strategy spec, then feed candles
/// one at a time with `step` and read the equity tail as you go — the same
/// engine and the same values as the batch `run`, driven bar-by-bar in the
/// browser (a live or replayed feed). The handle owns its spec, so it carries no
/// borrow across steps.
#[wasm_bindgen]
pub struct StreamingBacktest {
    inner: Option<CoreStreaming<'static>>,
}

#[wasm_bindgen]
impl StreamingBacktest {
    /// Build from a JSON strategy spec and starting capital.
    #[wasm_bindgen(constructor)]
    pub fn new(spec_json: &str, capital: f64) -> Result<StreamingBacktest, JsError> {
        let spec = StrategySpec::parse(spec_json).map_err(to_js)?;
        let inner = CoreStreaming::new_owned(spec, capital).map_err(to_js)?;
        Ok(Self { inner: Some(inner) })
    }

    fn engine(&self) -> Result<&CoreStreaming<'static>, JsError> {
        self.inner
            .as_ref()
            .ok_or_else(|| JsError::new("backtest already finished"))
    }

    /// Feed one bar (a JSON `Candle`).
    pub fn step(&mut self, candle_json: &str) -> Result<(), JsError> {
        let candle: Candle = serde_json::from_str(candle_json).map_err(to_js)?;
        self.inner
            .as_mut()
            .ok_or_else(|| JsError::new("backtest already finished"))?
            .step(&candle)
            .map_err(to_js)
    }

    /// Feed one bar plus a reference-series close (for pairwise indicators).
    #[wasm_bindgen(js_name = stepWithRef)]
    pub fn step_with_ref(&mut self, candle_json: &str, reference: f64) -> Result<(), JsError> {
        let candle: Candle = serde_json::from_str(candle_json).map_err(to_js)?;
        self.inner
            .as_mut()
            .ok_or_else(|| JsError::new("backtest already finished"))?
            .step_with_ref(&candle, Some(reference))
            .map_err(to_js)
    }

    /// The number of closed trades so far.
    #[wasm_bindgen(js_name = numTrades)]
    pub fn num_trades(&self) -> Result<usize, JsError> {
        Ok(self.engine()?.num_trades())
    }

    /// The equity curve so far as a JSON array (oldest first).
    pub fn equity(&self) -> Result<String, JsError> {
        serde_json::to_string(self.engine()?.equity()).map_err(to_js)
    }

    /// The latest equity point as JSON, or `null` before the first bar.
    #[wasm_bindgen(js_name = latestEquity)]
    pub fn latest_equity(&self) -> Result<String, JsError> {
        serde_json::to_string(&self.engine()?.latest_equity()).map_err(to_js)
    }

    /// Close any open position and produce the final `BacktestReport` as JSON.
    /// The handle is consumed; further calls error.
    pub fn finish(&mut self) -> Result<String, JsError> {
        let engine = self
            .inner
            .take()
            .ok_or_else(|| JsError::new("backtest already finished"))?;
        serde_json::to_string(&engine.finish()).map_err(to_js)
    }
}
