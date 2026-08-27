//! The backtest result: metrics, the trade log and the equity curve.

use serde::Serialize;

use crate::metrics::Metrics;
use crate::portfolio::Trade;

/// Current report schema version.
pub const REPORT_SCHEMA_VERSION: u32 = 1;

/// One point on the equity curve (marked at each bar close).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct EquityPoint {
    /// Bar time.
    pub time: i64,
    /// Mark-to-market equity at the bar close.
    pub equity: f64,
}

/// The result of a backtest run.
#[derive(Debug, Clone, Serialize)]
pub struct BacktestReport {
    /// Report schema version.
    pub schema_version: u32,
    /// The instrument the spec was written for, echoed from it.
    ///
    /// The engine reads whatever candles it is given and does not check them
    /// against this, so it is a label rather than a guarantee -- but without it a
    /// stored report does not say what it is a report of, which makes a directory
    /// of them unreadable.
    pub symbol: String,
    /// The bar size the spec was written for, echoed from it. A label, like
    /// `symbol`.
    pub timeframe: String,
    /// Summary metrics.
    pub metrics: Metrics,
    /// Completed trades.
    pub trades: Vec<Trade>,
    /// Per-bar equity curve.
    pub equity: Vec<EquityPoint>,
    /// Total fees paid.
    pub fees_paid: f64,
    /// Starting capital.
    pub initial_capital: f64,
}
