//! Indicator registry: constructs `wickra-core` indicators by name and wraps
//! them behind a uniform, object-safe [`EvalIndicator`] the engine can drive
//! from a [`Candle`].
//!
//! Single-output indicators expose one value; multi-output indicators (MACD,
//! Bollinger, Stochastic, …) also expose named fields, referenced in the spec
//! as `"name.field"`.
//!
//! NOTE: this is a curated hand-written registry. The full registry over every
//! `wickra-core` indicator is generated from the manifest (handoff-20 Phase 2
//! generator); this file is the seam it replaces.

use wickra_core::{
    Adx, Aroon, Atr, BollingerBands, Candle as CoreCandle, Cci, Cmo, Dema, Donchian, Ema, Hma,
    Indicator, Kama, Keltner, MacdIndicator, Mfi, Mom, Obv, Roc, Rsi, Sma, Stochastic, Tema, Trima,
    Trix, Vwap, WilliamsR, Wma,
};

use crate::data::Candle;
use crate::error::{BacktestError, Result};

/// A uniform, object-safe indicator the engine drives one bar at a time.
pub trait EvalIndicator: Send {
    /// Feed one bar; returns the primary value, or `None` while warming up.
    fn update(&mut self, candle: &Candle) -> Option<f64>;
    /// Named output fields of the most recent update (empty for single-output).
    fn fields(&self) -> Vec<(&'static str, f64)>;
    /// Number of bars required before the first value.
    fn warmup(&self) -> usize;
}

/// Wraps a scalar (`Input = f64`) single-output indicator, fed the bar close.
struct ScalarClose<I>(I);

impl<I> EvalIndicator for ScalarClose<I>
where
    I: Indicator<Input = f64, Output = f64> + Send,
{
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        self.0.update(candle.close)
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        Vec::new()
    }
    fn warmup(&self) -> usize {
        self.0.warmup_period()
    }
}

/// Wraps a candle (`Input = Candle`) single-output indicator.
struct CandleIn<I>(I);

impl<I> EvalIndicator for CandleIn<I>
where
    I: Indicator<Input = CoreCandle, Output = f64> + Send,
{
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        candle.to_core().ok().and_then(|c| self.0.update(c))
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        Vec::new()
    }
    fn warmup(&self) -> usize {
        self.0.warmup_period()
    }
}

/// MACD (`{macd, signal, histogram}`); primary value is the MACD line.
struct MacdWrap {
    inner: MacdIndicator,
    last: Vec<(&'static str, f64)>,
}

impl EvalIndicator for MacdWrap {
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        let out = self.inner.update(candle.close)?;
        self.last = vec![
            ("macd", out.macd),
            ("signal", out.signal),
            ("histogram", out.histogram),
        ];
        Some(out.macd)
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        self.last.clone()
    }
    fn warmup(&self) -> usize {
        self.inner.warmup_period()
    }
}

/// Bollinger Bands (`{upper, middle, lower}`); primary value is the middle band.
struct BollingerWrap {
    inner: BollingerBands,
    last: Vec<(&'static str, f64)>,
}

impl EvalIndicator for BollingerWrap {
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        let out = self.inner.update(candle.close)?;
        self.last = vec![
            ("upper", out.upper),
            ("middle", out.middle),
            ("lower", out.lower),
        ];
        Some(out.middle)
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        self.last.clone()
    }
    fn warmup(&self) -> usize {
        self.inner.warmup_period()
    }
}

/// Stochastic oscillator (`{k, d}`); primary value is `%K`.
struct StochasticWrap {
    inner: Stochastic,
    last: Vec<(&'static str, f64)>,
}

impl EvalIndicator for StochasticWrap {
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        let out = candle.to_core().ok().and_then(|c| self.inner.update(c))?;
        self.last = vec![("k", out.k), ("d", out.d)];
        Some(out.k)
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        self.last.clone()
    }
    fn warmup(&self) -> usize {
        self.inner.warmup_period()
    }
}

/// ADX (`{adx, plus_di, minus_di}`); primary value is the ADX line.
struct AdxWrap {
    inner: Adx,
    last: Vec<(&'static str, f64)>,
}

impl EvalIndicator for AdxWrap {
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        let out = candle.to_core().ok().and_then(|c| self.inner.update(c))?;
        self.last = vec![
            ("adx", out.adx),
            ("plus_di", out.plus_di),
            ("minus_di", out.minus_di),
        ];
        Some(out.adx)
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        self.last.clone()
    }
    fn warmup(&self) -> usize {
        self.inner.warmup_period()
    }
}

/// Aroon (`{up, down}`); primary value is Aroon-Up.
struct AroonWrap {
    inner: Aroon,
    last: Vec<(&'static str, f64)>,
}

impl EvalIndicator for AroonWrap {
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        let out = candle.to_core().ok().and_then(|c| self.inner.update(c))?;
        self.last = vec![("up", out.up), ("down", out.down)];
        Some(out.up)
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        self.last.clone()
    }
    fn warmup(&self) -> usize {
        self.inner.warmup_period()
    }
}

/// Keltner Channels (`{upper, middle, lower}`); primary value is the middle line.
struct KeltnerWrap {
    inner: Keltner,
    last: Vec<(&'static str, f64)>,
}

impl EvalIndicator for KeltnerWrap {
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        let out = candle.to_core().ok().and_then(|c| self.inner.update(c))?;
        self.last = vec![
            ("upper", out.upper),
            ("middle", out.middle),
            ("lower", out.lower),
        ];
        Some(out.middle)
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        self.last.clone()
    }
    fn warmup(&self) -> usize {
        self.inner.warmup_period()
    }
}

/// Donchian Channels (`{upper, middle, lower}`); primary value is the middle line.
struct DonchianWrap {
    inner: Donchian,
    last: Vec<(&'static str, f64)>,
}

impl EvalIndicator for DonchianWrap {
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        let out = candle.to_core().ok().and_then(|c| self.inner.update(c))?;
        self.last = vec![
            ("upper", out.upper),
            ("middle", out.middle),
            ("lower", out.lower),
        ];
        Some(out.middle)
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        self.last.clone()
    }
    fn warmup(&self) -> usize {
        self.inner.warmup_period()
    }
}

/// Read parameter `idx` as a positive-integer period.
fn period(params: &[f64], idx: usize, kind: &str) -> Result<usize> {
    let v = float_param(params, idx, kind)?;
    if v <= 0.0 || v.fract().abs() > f64::EPSILON {
        return Err(BacktestError::InvalidParams {
            indicator: kind.to_string(),
            reason: format!("parameter #{idx} must be a positive integer, got {v}"),
        });
    }
    Ok(v as usize)
}

/// Read parameter `idx` as a finite `f64`.
fn float_param(params: &[f64], idx: usize, kind: &str) -> Result<f64> {
    let v = params
        .get(idx)
        .copied()
        .ok_or_else(|| BacktestError::InvalidParams {
            indicator: kind.to_string(),
            reason: format!("missing parameter #{idx}"),
        })?;
    if !v.is_finite() {
        return Err(BacktestError::InvalidParams {
            indicator: kind.to_string(),
            reason: format!("parameter #{idx} must be finite"),
        });
    }
    Ok(v)
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
    let p = |i| period(params, i, kind);
    match kind {
        // Scalar single-output (fed the close).
        "Sma" => Ok(Box::new(ScalarClose(map_new(kind, Sma::new(p(0)?))?))),
        "Ema" => Ok(Box::new(ScalarClose(map_new(kind, Ema::new(p(0)?))?))),
        "Wma" => Ok(Box::new(ScalarClose(map_new(kind, Wma::new(p(0)?))?))),
        "Dema" => Ok(Box::new(ScalarClose(map_new(kind, Dema::new(p(0)?))?))),
        "Tema" => Ok(Box::new(ScalarClose(map_new(kind, Tema::new(p(0)?))?))),
        "Hma" => Ok(Box::new(ScalarClose(map_new(kind, Hma::new(p(0)?))?))),
        "Rsi" => Ok(Box::new(ScalarClose(map_new(kind, Rsi::new(p(0)?))?))),
        "Roc" => Ok(Box::new(ScalarClose(map_new(kind, Roc::new(p(0)?))?))),
        "Mom" => Ok(Box::new(ScalarClose(map_new(kind, Mom::new(p(0)?))?))),
        // Candle single-output.
        "Atr" => Ok(Box::new(CandleIn(map_new(kind, Atr::new(p(0)?))?))),
        // Multi-output.
        "Macd" => Ok(Box::new(MacdWrap {
            inner: map_new(kind, MacdIndicator::new(p(0)?, p(1)?, p(2)?))?,
            last: Vec::new(),
        })),
        "Bollinger" => Ok(Box::new(BollingerWrap {
            inner: map_new(
                kind,
                BollingerBands::new(p(0)?, float_param(params, 1, kind)?),
            )?,
            last: Vec::new(),
        })),
        "Stochastic" => Ok(Box::new(StochasticWrap {
            inner: map_new(kind, Stochastic::new(p(0)?, p(1)?))?,
            last: Vec::new(),
        })),
        // More scalar single-output.
        "Cmo" => Ok(Box::new(ScalarClose(map_new(kind, Cmo::new(p(0)?))?))),
        "Trix" => Ok(Box::new(ScalarClose(map_new(kind, Trix::new(p(0)?))?))),
        "Trima" => Ok(Box::new(ScalarClose(map_new(kind, Trima::new(p(0)?))?))),
        "Kama" => Ok(Box::new(ScalarClose(map_new(
            kind,
            Kama::new(p(0)?, p(1)?, p(2)?),
        )?))),
        // More candle single-output.
        "Cci" => Ok(Box::new(CandleIn(map_new(kind, Cci::new(p(0)?))?))),
        "WilliamsR" => Ok(Box::new(CandleIn(map_new(kind, WilliamsR::new(p(0)?))?))),
        "Mfi" => Ok(Box::new(CandleIn(map_new(kind, Mfi::new(p(0)?))?))),
        "Vwap" => Ok(Box::new(CandleIn(Vwap::new()))),
        "Obv" => Ok(Box::new(CandleIn(Obv::new()))),
        // More multi-output (candle).
        "Adx" => Ok(Box::new(AdxWrap {
            inner: map_new(kind, Adx::new(p(0)?))?,
            last: Vec::new(),
        })),
        "Aroon" => Ok(Box::new(AroonWrap {
            inner: map_new(kind, Aroon::new(p(0)?))?,
            last: Vec::new(),
        })),
        "Keltner" => Ok(Box::new(KeltnerWrap {
            inner: map_new(
                kind,
                Keltner::new(p(0)?, p(1)?, float_param(params, 2, kind)?),
            )?,
            last: Vec::new(),
        })),
        "Donchian" => Ok(Box::new(DonchianWrap {
            inner: map_new(kind, Donchian::new(p(0)?))?,
            last: Vec::new(),
        })),
        other => Err(BacktestError::UnknownIndicator(other.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(high: f64, low: f64, close: f64) -> Candle {
        Candle {
            time: 0,
            open: close,
            high,
            low,
            close,
            volume: 1.0,
        }
    }

    #[test]
    fn builds_all_known_indicators() {
        let specs: &[(&str, &[f64])] = &[
            ("Sma", &[14.0]),
            ("Ema", &[14.0]),
            ("Wma", &[14.0]),
            ("Dema", &[14.0]),
            ("Tema", &[14.0]),
            ("Hma", &[14.0]),
            ("Rsi", &[14.0]),
            ("Roc", &[14.0]),
            ("Mom", &[14.0]),
            ("Atr", &[14.0]),
            ("Macd", &[12.0, 26.0, 9.0]),
            ("Bollinger", &[20.0, 2.0]),
            ("Stochastic", &[14.0, 3.0]),
            ("Cmo", &[14.0]),
            ("Trix", &[14.0]),
            ("Trima", &[14.0]),
            ("Kama", &[10.0, 2.0, 30.0]),
            ("Cci", &[20.0]),
            ("WilliamsR", &[14.0]),
            ("Mfi", &[14.0]),
            ("Vwap", &[]),
            ("Obv", &[]),
            ("Adx", &[14.0]),
            ("Aroon", &[14.0]),
            ("Keltner", &[20.0, 10.0, 2.0]),
            ("Donchian", &[20.0]),
        ];
        for (kind, params) in specs {
            assert!(build(kind, params).is_ok(), "{kind} should build");
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
        assert!(build("Macd", &[12.0, 26.0]).is_err()); // missing signal
        assert!(build("Bollinger", &[20.0]).is_err()); // missing multiplier
    }

    #[test]
    fn macd_exposes_fields() {
        let mut macd = build("Macd", &[2.0, 3.0, 2.0]).unwrap();
        // feed enough bars to warm up, then check named fields appear.
        let mut last_fields = Vec::new();
        for px in [10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0] {
            if macd.update(&candle(px, px, px)).is_some() {
                last_fields = macd.fields();
            }
        }
        let names: Vec<&str> = last_fields.iter().map(|(n, _)| *n).collect();
        assert!(
            names.contains(&"macd") && names.contains(&"signal") && names.contains(&"histogram")
        );
    }

    #[test]
    fn single_output_has_no_fields() {
        let mut sma = build("Sma", &[2.0]).unwrap();
        sma.update(&candle(10.0, 10.0, 10.0));
        sma.update(&candle(20.0, 20.0, 20.0));
        assert!(sma.fields().is_empty());
    }
}
