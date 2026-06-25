//! Python bindings for `wickra-backtest`.
//!
//! Exposes a single `run(...)` that takes parallel `OHLCV` arrays and a JSON
//! strategy spec and returns the `BacktestReport` as a JSON string. The Python
//! wrapper (`python/wickra_backtest/__init__.py`) turns that into a dict and
//! accepts lists or `NumPy` arrays.

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

use wickra_backtest_core::{run_with_capital, Candle, StrategySpec};

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

#[pymodule]
fn _wickra_backtest(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_function(wrap_pyfunction!(run, m)?)?;
    Ok(())
}
