//! Market-data input types fed to the engine.
//!
//! These are serde-friendly, owned value types (loadable from JSONL/CSV/Parquet)
//! that convert into the `wickra-core` input types when fed to indicators.

use serde::{Deserialize, Serialize};
use wickra_core::Candle as CoreCandle;

use crate::error::{BacktestError, Result};

/// One OHLCV bar. `time` is the bar's open time (engine-defined epoch unit; it is
/// passed straight through to indicators that need a timestamp).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Candle {
    /// Bar open time (epoch, engine-defined unit — seconds by convention).
    pub time: i64,
    /// Open price.
    pub open: f64,
    /// High price.
    pub high: f64,
    /// Low price.
    pub low: f64,
    /// Close price.
    pub close: f64,
    /// Bar volume (defaults to `0.0` when absent).
    #[serde(default)]
    pub volume: f64,
}

impl Candle {
    /// Convert into a `wickra-core` candle for feeding indicators. Fails if the
    /// OHLC values are not finite or violate `high >= low` etc.
    pub fn to_core(self) -> Result<CoreCandle> {
        CoreCandle::new(
            self.open,
            self.high,
            self.low,
            self.close,
            self.volume,
            self.time,
        )
        .map_err(|e| BacktestError::InvalidData(e.to_string()))
    }

    /// Typical price `(high + low + close) / 3`.
    #[must_use]
    pub fn hlc3(self) -> f64 {
        (self.high + self.low + self.close) / 3.0
    }

    /// Average price `(open + high + low + close) / 4`.
    #[must_use]
    pub fn ohlc4(self) -> f64 {
        (self.open + self.high + self.low + self.close) / 4.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_to_core() {
        let c = Candle {
            time: 1,
            open: 10.0,
            high: 12.0,
            low: 9.0,
            close: 11.0,
            volume: 100.0,
        };
        assert!(c.to_core().is_ok());
    }

    #[test]
    fn rejects_non_finite() {
        let c = Candle {
            time: 1,
            open: f64::NAN,
            high: 1.0,
            low: 1.0,
            close: 1.0,
            volume: 0.0,
        };
        assert!(c.to_core().is_err());
    }

    #[test]
    fn derived_prices() {
        let c = Candle {
            time: 0,
            open: 4.0,
            high: 6.0,
            low: 2.0,
            close: 4.0,
            volume: 0.0,
        };
        assert!((c.hlc3() - 4.0).abs() < 1e-12);
        assert!((c.ohlc4() - 4.0).abs() < 1e-12);
    }

    #[test]
    fn volume_defaults_to_zero() {
        let c: Candle =
            serde_json::from_str(r#"{"time":0,"open":1,"high":1,"low":1,"close":1}"#).unwrap();
        assert!(c.volume.abs() < f64::EPSILON);
    }
}
