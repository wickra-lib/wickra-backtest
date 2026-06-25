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
