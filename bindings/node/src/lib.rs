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
    run_json as core_run_json, run_with_capital, Candle, StepRequest, StrategySpec,
    StreamingBacktest as CoreStreaming, DEFAULT_CAPITAL,
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

/// A backtest driven one bar at a time.
///
/// `run` needs the whole series up front; this drives the same engine bar by
/// bar, so a live loop and a backtest are the same code path -- feed it from a
/// socket instead of from an array and every value it reports was produced the
/// way the backtest produced it.
///
/// Like `run`, the reporting methods return JSON strings for the caller to
/// `JSON.parse`, rather than mirroring the whole report shape in napi objects.
///
/// `finish` consumes the run, which a `&mut self` receiver cannot express, so
/// the engine is held in an `Option` and taken on finish. Using the object
/// afterwards throws instead of resurrecting a half-finished run.
// Named for JS directly rather than via `js_name` on a differently named struct:
// that spelling makes napi emit a second runtime export for the Rust name, so
// the package would carry two names for one class.
//
// CodeQL's `rust/access-invalid-pointer` fires on the expansion below and is a
// false positive, dismissed as such. The derive emits the standard N-API unwrap:
// a pointer is initialised to null, written by `napi_unwrap` across the FFI
// boundary, then dereferenced. CodeQL cannot follow a write through an extern
// call, so it sees a null deref that cannot happen. This is the first `#[napi]`
// struct here; the indicator repository has hundreds without the alert, because
// there they come from a `macro_rules!` body the Rust extractor treats
// differently. Re-open the alert if napi-rs changes how it expands classes.
#[napi]
#[derive(Debug)]
pub struct StreamingBacktest {
    inner: Option<CoreStreaming<'static>>,
    /// Bars fed so far, used as the default timestamp. Kept as a signed counter
    /// rather than reading the equity length, which would need a `usize` cast.
    bars: i64,
}

impl StreamingBacktest {
    fn engine(&self) -> Result<&CoreStreaming<'static>> {
        self.inner
            .as_ref()
            .ok_or_else(|| Error::from_reason("this backtest is finished"))
    }

    fn engine_mut(&mut self) -> Result<&mut CoreStreaming<'static>> {
        self.inner
            .as_mut()
            .ok_or_else(|| Error::from_reason("this backtest is finished"))
    }
}

#[napi]
impl StreamingBacktest {
    #[napi(constructor)]
    pub fn new(spec_json: String, capital: Option<f64>) -> Result<Self> {
        let spec = StrategySpec::parse(&spec_json).map_err(to_napi)?;
        let inner =
            CoreStreaming::new_owned(spec, capital.unwrap_or(DEFAULT_CAPITAL)).map_err(to_napi)?;
        Ok(Self {
            inner: Some(inner),
            bars: 0,
        })
    }

    /// Advance by one OHLCV bar. `volume` defaults to 0 and `time` to the number
    /// of bars fed so far, matching `run`'s default of `range(len)`.
    #[napi]
    pub fn step(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: Option<f64>,
        time: Option<f64>,
    ) -> Result<()> {
        let bars = self.bars;
        let candle = Candle {
            time: time.map_or(bars, |t| t as i64),
            open,
            high,
            low,
            close,
            volume: volume.unwrap_or(0.0),
        };
        self.engine_mut()?.step(&candle).map_err(to_napi)?;
        self.bars += 1;
        Ok(())
    }

    /// Advance by one bar described as a request document:
    /// `{"candle": {...}, "feeds": {...}}`, where `feeds` optionally carries this
    /// bar's `reference` / `deriv` / `orderbook` / `trades` / `cross_section`.
    /// This is the only form that can drive a strategy reading a side feed.
    #[napi]
    pub fn step_json(&mut self, step_json: String) -> Result<()> {
        let step: StepRequest = serde_json::from_str(&step_json).map_err(to_napi)?;
        self.engine_mut()?
            .step_with_feeds(&step.candle, &step.feeds.as_feeds())
            .map_err(to_napi)?;
        self.bars += 1;
        Ok(())
    }

    /// The equity curve so far, as a JSON array.
    #[napi]
    pub fn equity_json(&self) -> Result<String> {
        serde_json::to_string(self.engine()?.equity()).map_err(to_napi)
    }

    /// The most recent equity point as JSON, or `null` before the first bar.
    #[napi]
    pub fn latest_equity_json(&self) -> Result<String> {
        serde_json::to_string(&self.engine()?.latest_equity()).map_err(to_napi)
    }

    /// The number of closed trades so far.
    #[napi(getter)]
    pub fn num_trades(&self) -> Result<u32> {
        Ok(self.engine()?.num_trades() as u32)
    }

    /// Whether the run has been finished or closed.
    #[napi(getter)]
    pub fn is_finished(&self) -> bool {
        self.inner.is_none()
    }

    /// Close any open position and return the report JSON. Ends the run.
    #[napi]
    pub fn finish_json(&mut self) -> Result<String> {
        let inner = self
            .inner
            .take()
            .ok_or_else(|| Error::from_reason("this backtest is finished"))?;
        serde_json::to_string(&inner.finish()).map_err(to_napi)
    }

    /// Drop the run without producing a report. Idempotent.
    #[napi]
    pub fn close(&mut self) {
        self.inner = None;
    }
}
