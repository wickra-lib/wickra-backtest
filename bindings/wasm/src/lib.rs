//! WebAssembly bindings for `wickra-backtest` — run a backtest in the browser.
//!
//! `run(...)` takes parallel `Float64Array`s and a JSON strategy spec and
//! returns the `BacktestReport` as a JSON string (the caller `JSON.parse`s it).
//! Pairs with `live.wickra.org`: backtest a strategy entirely client-side, with
//! the same kernel and the same values as the Rust, Python and Node bindings.

use wasm_bindgen::prelude::*;

use wickra_backtest_core::{
    run_json as core_run_json, run_with_capital, Candle, StepRequest, StrategySpec,
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

/// A backtest driven one bar at a time.
///
/// `run` needs the whole series up front; this drives the same engine bar by
/// bar, so a live loop and a backtest are the same code path -- point it at a
/// websocket instead of an array and every value it reports was produced the way
/// the backtest produced it.
///
/// The method names match the Node binding deliberately. Both are consumed from
/// JavaScript, and a developer moving between the npm package and this one
/// should not have to relearn the surface; the reporting methods carry the
/// `Json` suffix because, as in Node, they return JSON strings to parse.
///
/// The handle owns its spec, so it carries no borrow across steps. `finishJson`
/// consumes the run; `close` releases one without a report. Either leaves the
/// object inert, and wasm-bindgen's generated `free` reclaims the memory.
#[wasm_bindgen]
#[derive(Debug)]
pub struct StreamingBacktest {
    inner: Option<CoreStreaming<'static>>,
    bars: i64,
}

#[wasm_bindgen]
impl StreamingBacktest {
    /// Build from a JSON strategy spec and starting capital.
    #[wasm_bindgen(constructor)]
    pub fn new(spec_json: &str, capital: f64) -> Result<StreamingBacktest, JsError> {
        let spec = StrategySpec::parse(spec_json).map_err(to_js)?;
        let inner = CoreStreaming::new_owned(spec, capital).map_err(to_js)?;
        Ok(Self {
            inner: Some(inner),
            bars: 0,
        })
    }

    fn engine(&self) -> Result<&CoreStreaming<'static>, JsError> {
        self.inner
            .as_ref()
            .ok_or_else(|| JsError::new("this backtest is finished"))
    }

    fn engine_mut(&mut self) -> Result<&mut CoreStreaming<'static>, JsError> {
        self.inner
            .as_mut()
            .ok_or_else(|| JsError::new("this backtest is finished"))
    }

    /// Advance by one OHLCV bar. `volume` defaults to 0 and `time` to the number
    /// of bars fed so far.
    pub fn step(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: Option<f64>,
        time: Option<f64>,
    ) -> Result<(), JsError> {
        // Timestamps arrive as doubles: JavaScript numbers, and WASM has no
        // native i64 across the boundary.
        let bars = self.bars;
        let candle = Candle {
            time: time.map_or(bars, |t| t as i64),
            open,
            high,
            low,
            close,
            volume: volume.unwrap_or(0.0),
        };
        self.engine_mut()?.step(&candle).map_err(to_js)?;
        self.bars += 1;
        Ok(())
    }

    /// Advance by one bar described as a request document:
    /// `{"candle": {...}, "feeds": {...}}`, where `feeds` optionally carries this
    /// bar's `reference` / `deriv` / `orderbook` / `trades` / `cross_section`.
    /// This is the only form that can drive a strategy reading a side feed.
    #[wasm_bindgen(js_name = stepJson)]
    pub fn step_json(&mut self, step_json: &str) -> Result<(), JsError> {
        let step: StepRequest = serde_json::from_str(step_json).map_err(to_js)?;
        self.engine_mut()?
            .step_with_feeds(&step.candle, &step.feeds.as_feeds())
            .map_err(to_js)?;
        self.bars += 1;
        Ok(())
    }

    /// The number of closed trades so far.
    #[wasm_bindgen(getter, js_name = numTrades)]
    pub fn num_trades(&self) -> Result<usize, JsError> {
        Ok(self.engine()?.num_trades())
    }

    /// Whether the run has been finished or closed.
    #[wasm_bindgen(getter, js_name = isFinished)]
    pub fn is_finished(&self) -> bool {
        self.inner.is_none()
    }

    /// The equity curve so far, as a JSON array.
    #[wasm_bindgen(js_name = equityJson)]
    pub fn equity_json(&self) -> Result<String, JsError> {
        serde_json::to_string(self.engine()?.equity()).map_err(to_js)
    }

    /// The most recent equity point as JSON, or `null` before the first bar.
    #[wasm_bindgen(js_name = latestEquityJson)]
    pub fn latest_equity_json(&self) -> Result<String, JsError> {
        serde_json::to_string(&self.engine()?.latest_equity()).map_err(to_js)
    }

    /// Close any open position and return the report JSON. Ends the run.
    #[wasm_bindgen(js_name = finishJson)]
    pub fn finish_json(&mut self) -> Result<String, JsError> {
        let engine = self
            .inner
            .take()
            .ok_or_else(|| JsError::new("this backtest is finished"))?;
        serde_json::to_string(&engine.finish()).map_err(to_js)
    }

    /// Drop the run without producing a report. Idempotent.
    pub fn close(&mut self) {
        self.inner = None;
    }
}
