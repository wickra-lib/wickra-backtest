//! Python bindings for `wickra-backtest`.
//!
//! Exposes a single `run(...)` that takes parallel `OHLCV` arrays and a JSON
//! strategy spec and returns the `BacktestReport` as a JSON string. The Python
//! wrapper (`python/wickra_backtest/__init__.py`) turns that into a dict and
//! accepts lists or `NumPy` arrays.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use wickra_backtest_core::{
    run_json as core_run_json, run_with_capital, Candle, StepRequest, StrategySpec,
    StreamingBacktest as CoreStreaming,
};

/// Run a backtest. Returns the report as a JSON string.
#[pyfunction]
#[pyo3(signature = (open, high, low, close, volume, time, spec_json, capital = 10_000.0))]
#[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
fn run(
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    time: Vec<i64>,
    spec_json: &str,
    capital: f64,
) -> PyResult<String> {
    let n = open.len();
    for (name, len) in [
        ("high", high.len()),
        ("low", low.len()),
        ("close", close.len()),
        ("volume", volume.len()),
        ("time", time.len()),
    ] {
        if len != n {
            return Err(PyValueError::new_err(format!(
                "{name} length {len} does not match open length {n}"
            )));
        }
    }

    let candles: Vec<Candle> = (0..n)
        .map(|i| Candle {
            time: time[i],
            open: open[i],
            high: high[i],
            low: low[i],
            close: close[i],
            volume: volume[i],
        })
        .collect();

    let spec = StrategySpec::parse(spec_json).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let report = run_with_capital(&spec, &candles, capital)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    serde_json::to_string(&report).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Run a backtest from a single JSON request bundling candles, spec and optional
/// feeds. Returns the report as a JSON string.
#[pyfunction]
fn run_json(request_json: &str) -> PyResult<String> {
    core_run_json(request_json).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// A backtest driven one bar at a time.
///
/// `run` needs the whole series up front; this drives the same engine bar by
/// bar, so a live loop and a backtest are the same code path -- feed it from a
/// socket instead of from an array and every value it reports was produced the
/// way the backtest produced it.
///
/// `finish` consumes the run, which a `#[pymethods]` receiver cannot express, so
/// the engine is held in an `Option` and taken on finish. Calling anything
/// afterwards raises rather than resurrecting a half-finished run.
#[pyclass(
    name = "StreamingBacktest",
    module = "wickra_backtest._wickra_backtest"
)]
struct PyStreamingBacktest {
    inner: Option<CoreStreaming<'static>>,
}

impl PyStreamingBacktest {
    /// The live engine, or an error naming the mistake if the run is over.
    fn engine(&self) -> PyResult<&CoreStreaming<'static>> {
        self.inner
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("this backtest is finished"))
    }

    fn engine_mut(&mut self) -> PyResult<&mut CoreStreaming<'static>> {
        self.inner
            .as_mut()
            .ok_or_else(|| PyValueError::new_err("this backtest is finished"))
    }
}

#[pymethods]
impl PyStreamingBacktest {
    #[new]
    #[pyo3(signature = (spec_json, capital = 10_000.0))]
    fn new(spec_json: &str, capital: f64) -> PyResult<Self> {
        let spec =
            StrategySpec::parse(spec_json).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let inner = CoreStreaming::new_owned(spec, capital)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        Ok(Self { inner: Some(inner) })
    }

    /// Advance by one OHLCV bar.
    fn step(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        time: i64,
    ) -> PyResult<()> {
        let candle = Candle {
            time,
            open,
            high,
            low,
            close,
            volume,
        };
        self.engine_mut()?
            .step(&candle)
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Advance by one bar described as a `StepRequest` document, which is the
    /// only form that can also carry the bar's reference / derivatives /
    /// order-book / trade / cross-section feeds.
    fn step_json(&mut self, step_json: &str) -> PyResult<()> {
        let step: StepRequest =
            serde_json::from_str(step_json).map_err(|e| PyValueError::new_err(e.to_string()))?;
        self.engine_mut()?
            .step_with_feeds(&step.candle, &step.feeds.as_feeds())
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// The equity curve so far, as a JSON array.
    fn equity_json(&self) -> PyResult<String> {
        serde_json::to_string(self.engine()?.equity())
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// The most recent equity point as JSON, or `null` before the first bar.
    fn latest_equity_json(&self) -> PyResult<String> {
        serde_json::to_string(&self.engine()?.latest_equity())
            .map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// The number of closed trades so far.
    #[getter]
    fn num_trades(&self) -> PyResult<usize> {
        Ok(self.engine()?.num_trades())
    }

    /// Close any open position and return the report JSON. Consumes the run.
    fn finish_json(&mut self) -> PyResult<String> {
        let inner = self
            .inner
            .take()
            .ok_or_else(|| PyValueError::new_err("this backtest is finished"))?;
        serde_json::to_string(&inner.finish()).map_err(|e| PyValueError::new_err(e.to_string()))
    }

    /// Drop the run without producing a report. Idempotent.
    fn close(&mut self) {
        self.inner = None;
    }

    #[getter]
    fn is_finished(&self) -> bool {
        self.inner.is_none()
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            Some(bt) => format!(
                "StreamingBacktest(bars={}, trades={})",
                bt.equity().len(),
                bt.num_trades()
            ),
            None => "StreamingBacktest(finished)".to_string(),
        }
    }
}

#[pymodule]
fn _wickra_backtest(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(run, m)?)?;
    m.add_function(wrap_pyfunction!(run_json, m)?)?;
    m.add_class::<PyStreamingBacktest>()?;
    Ok(())
}
