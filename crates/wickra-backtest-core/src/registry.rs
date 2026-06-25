//! Indicator registry: constructs `wickra-core` indicators by name and wraps
//! them behind a uniform, object-safe [`EvalIndicator`] the engine can drive
//! from a [`Candle`].
//!
//! NOTE: this is the **hand-written MVP registry** (Sma / Ema / Rsi / Atr). The
//! full registry over every `wickra-core` indicator is generated from the
//! manifest (handoff-20 Phase 2 generator); this file is the seam it replaces.

use wickra_core::{Atr, Candle as CoreCandle, Ema, Indicator, Rsi, Sma};

use crate::data::Candle;
use crate::error::{BacktestError, Result};

/// A uniform, object-safe indicator the engine drives one bar at a time.
pub trait EvalIndicator: Send {
    /// Feed one bar; returns the freshly computed value, or `None` while warming up.
    fn update(&mut self, candle: &Candle) -> Option<f64>;
    /// Number of bars required before the first value.
    fn warmup(&self) -> usize;
}

/// Wraps a scalar (`Input = f64`) indicator, feeding it the bar close.
struct ScalarClose<I>(I);

impl<I> EvalIndicator for ScalarClose<I>
where
    I: Indicator<Input = f64, Output = f64> + Send,
{
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        self.0.update(candle.close)
    }
    fn warmup(&self) -> usize {
        self.0.warmup_period()
    }
}

/// Wraps a candle (`Input = Candle`) indicator.
struct CandleIn<I>(I);

impl<I> EvalIndicator for CandleIn<I>
where
    I: Indicator<Input = CoreCandle, Output = f64> + Send,
{
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        candle.to_core().ok().and_then(|c| self.0.update(c))
    }
    fn warmup(&self) -> usize {
        self.0.warmup_period()
    }
}

/// Read parameter `idx` as a positive-integer period.
fn period(params: &[f64], idx: usize, kind: &str) -> Result<usize> {
    let v = params
        .get(idx)
        .copied()
        .ok_or_else(|| BacktestError::InvalidParams {
            indicator: kind.to_string(),
            reason: format!("missing parameter #{idx}"),
        })?;
    if v <= 0.0 || v.fract().abs() > f64::EPSILON {
        return Err(BacktestError::InvalidParams {
            indicator: kind.to_string(),
            reason: format!("parameter #{idx} must be a positive integer, got {v}"),
        });
    }
    Ok(v as usize)
}

/// Map a `wickra-core` constructor error into a [`BacktestError`].
fn map_new<T>(kind: &str, r: wickra_core::Result<T>) -> Result<T> {
    r.map_err(|e| BacktestError::InvalidParams {
        indicator: kind.to_string(),
        reason: e.to_string(),
    })
}

/// Construct an indicator by its `wickra-core` type name.
pub fn build(kind: &str, params: &[f64]) -> Result<Box<dyn EvalIndicator>> {
    match kind {
        "Sma" => Ok(Box::new(ScalarClose(map_new(
            kind,
            Sma::new(period(params, 0, kind)?),
        )?))),
        "Ema" => Ok(Box::new(ScalarClose(map_new(
            kind,
            Ema::new(period(params, 0, kind)?),
        )?))),
        "Rsi" => Ok(Box::new(ScalarClose(map_new(
            kind,
            Rsi::new(period(params, 0, kind)?),
        )?))),
        "Atr" => Ok(Box::new(CandleIn(map_new(
            kind,
            Atr::new(period(params, 0, kind)?),
        )?))),
        other => Err(BacktestError::UnknownIndicator(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(close: f64) -> Candle {
        Candle {
            time: 0,
            open: close,
            high: close,
            low: close,
            close,
            volume: 1.0,
        }
    }

    #[test]
    fn builds_known_indicators() {
        for kind in ["Sma", "Ema", "Rsi", "Atr"] {
            assert!(build(kind, &[14.0]).is_ok(), "{kind} should build");
        }
    }

    #[test]
    fn unknown_indicator_errors() {
        assert!(matches!(
            build("Nope", &[1.0]),
            Err(BacktestError::UnknownIndicator(_))
        ));
    }

    #[test]
    fn rejects_bad_period() {
        assert!(build("Sma", &[]).is_err());
        assert!(build("Sma", &[0.0]).is_err());
        assert!(build("Sma", &[2.5]).is_err());
    }

    #[test]
    fn sma_warms_up_and_values() {
        let mut sma = build("Sma", &[2.0]).unwrap();
        assert_eq!(sma.warmup(), 2);
        assert!(sma.update(&candle(10.0)).is_none());
        assert_eq!(sma.update(&candle(20.0)), Some(15.0));
    }
}
