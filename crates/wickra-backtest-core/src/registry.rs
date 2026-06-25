//! Indicator registry: constructs `wickra-core` indicators by name and wraps
//! them behind a uniform, object-safe [`EvalIndicator`] the engine can drive
//! from a [`Candle`].
//!
//! GENERATED FILE — do not edit by hand. Regenerate with:
//!
//! ```text
//! python tools/gen_registry.py --wickra ../wickra --out crates/wickra-backtest-core/src/registry.rs
//! ```
//!
//! Source of truth: the wickra wasm binding macros (exact Rust constructor
//! signatures) joined with the golden manifest (default parameters). Scalar
//! (`Input = f64`) and candlestick-pattern (`Input = Candle`, param-less)
//! indicators are generated; a few candle-input scalar indicators and the
//! multi-output indicators are kept hand-written.

use wickra_core::{self as wc, Candle as CoreCandle, Indicator};

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
    inner: wc::MacdIndicator,
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
    inner: wc::BollingerBands,
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
    inner: wc::Stochastic,
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
    inner: wc::Adx,
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
    inner: wc::Aroon,
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
    inner: wc::Keltner,
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
    inner: wc::Donchian,
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

/// Read parameter `idx` as a non-negative `u32`.
fn u32_param(params: &[f64], idx: usize, kind: &str) -> Result<u32> {
    let v = float_param(params, idx, kind)?;
    if v < 0.0 || v.fract().abs() > f64::EPSILON || v > f64::from(u32::MAX) {
        return Err(BacktestError::InvalidParams {
            indicator: kind.to_string(),
            reason: format!("parameter #{idx} must be a u32, got {v}"),
        });
    }
    Ok(v as u32)
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
fn map_new<T>(kind: &str, r: wc::Result<T>) -> Result<T> {
    r.map_err(|e| BacktestError::InvalidParams {
        indicator: kind.to_string(),
        reason: e.to_string(),
    })
}

/// Construct an indicator by its `wickra-core` type name.
#[allow(clippy::too_many_lines)]
pub fn build(kind: &str, params: &[f64]) -> Result<Box<dyn EvalIndicator>> {
    let p = |i| period(params, i, kind);
    let _ = &p;
    match kind {
        // --- generated scalar (Input = f64), fed the close ---
        "Sma" => Ok(Box::new(ScalarClose(map_new(kind, wc::Sma::new(p(0)?))?))),
        "Ema" => Ok(Box::new(ScalarClose(map_new(kind, wc::Ema::new(p(0)?))?))),
        "Wma" => Ok(Box::new(ScalarClose(map_new(kind, wc::Wma::new(p(0)?))?))),
        "Rsi" => Ok(Box::new(ScalarClose(map_new(kind, wc::Rsi::new(p(0)?))?))),
        "Dema" => Ok(Box::new(ScalarClose(map_new(kind, wc::Dema::new(p(0)?))?))),
        "Tema" => Ok(Box::new(ScalarClose(map_new(kind, wc::Tema::new(p(0)?))?))),
        "Hma" => Ok(Box::new(ScalarClose(map_new(kind, wc::Hma::new(p(0)?))?))),
        "Roc" => Ok(Box::new(ScalarClose(map_new(kind, wc::Roc::new(p(0)?))?))),
        "Trix" => Ok(Box::new(ScalarClose(map_new(kind, wc::Trix::new(p(0)?))?))),
        "Smma" => Ok(Box::new(ScalarClose(map_new(kind, wc::Smma::new(p(0)?))?))),
        "Trima" => Ok(Box::new(ScalarClose(map_new(kind, wc::Trima::new(p(0)?))?))),
        "Zlema" => Ok(Box::new(ScalarClose(map_new(kind, wc::Zlema::new(p(0)?))?))),
        "T3" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::T3::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "Alma" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Alma::new(
                p(0)?,
                float_param(params, 1, kind)?,
                float_param(params, 2, kind)?,
            ),
        )?))),
        "PolarizedFractalEfficiency" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::PolarizedFractalEfficiency::new(p(0)?, p(1)?),
        )?))),
        "WavePm" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::WavePm::new(p(0)?, p(1)?),
        )?))),
        "McGinleyDynamic" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::McGinleyDynamic::new(p(0)?),
        )?))),
        "Frama" => Ok(Box::new(ScalarClose(map_new(kind, wc::Frama::new(p(0)?))?))),
        "Vidya" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Vidya::new(p(0)?, p(1)?),
        )?))),
        "Jma" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Jma::new(
                p(0)?,
                float_param(params, 1, kind)?,
                u32_param(params, 2, kind)?,
            ),
        )?))),
        "Mom" => Ok(Box::new(ScalarClose(map_new(kind, wc::Mom::new(p(0)?))?))),
        "Cmo" => Ok(Box::new(ScalarClose(map_new(kind, wc::Cmo::new(p(0)?))?))),
        "Tsi" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Tsi::new(p(0)?, p(1)?),
        )?))),
        "Pmo" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Pmo::new(p(0)?, p(1)?),
        )?))),
        "Tii" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Tii::new(p(0)?, p(1)?),
        )?))),
        "StochRsi" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::StochRsi::new(p(0)?, p(1)?),
        )?))),
        "Dpo" => Ok(Box::new(ScalarClose(map_new(kind, wc::Dpo::new(p(0)?))?))),
        "Ppo" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Ppo::new(p(0)?, p(1)?),
        )?))),
        "Apo" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Apo::new(p(0)?, p(1)?),
        )?))),
        "Cfo" => Ok(Box::new(ScalarClose(map_new(kind, wc::Cfo::new(p(0)?))?))),
        "ElderImpulse" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::ElderImpulse::new(p(0)?, p(1)?, p(2)?, p(3)?),
        )?))),
        "Stc" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Stc::new(p(0)?, p(1)?, p(2)?, float_param(params, 3, kind)?),
        )?))),
        "Coppock" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Coppock::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "StdDev" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::StdDev::new(p(0)?),
        )?))),
        "UlcerIndex" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::UlcerIndex::new(p(0)?),
        )?))),
        "HistoricalVolatility" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::HistoricalVolatility::new(p(0)?, p(1)?),
        )?))),
        "BollingerBandwidth" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::BollingerBandwidth::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "PercentB" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::PercentB::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "LinearRegression" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::LinearRegression::new(p(0)?),
        )?))),
        "LinRegSlope" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::LinRegSlope::new(p(0)?),
        )?))),
        "VerticalHorizontalFilter" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::VerticalHorizontalFilter::new(p(0)?),
        )?))),
        "ZScore" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::ZScore::new(p(0)?),
        )?))),
        "LinRegAngle" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::LinRegAngle::new(p(0)?),
        )?))),
        "Variance" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Variance::new(p(0)?),
        )?))),
        "CoefficientOfVariation" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::CoefficientOfVariation::new(p(0)?),
        )?))),
        "Skewness" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Skewness::new(p(0)?),
        )?))),
        "Kurtosis" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Kurtosis::new(p(0)?),
        )?))),
        "StandardError" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::StandardError::new(p(0)?),
        )?))),
        "DetrendedStdDev" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::DetrendedStdDev::new(p(0)?),
        )?))),
        "RSquared" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RSquared::new(p(0)?),
        )?))),
        "MedianAbsoluteDeviation" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::MedianAbsoluteDeviation::new(p(0)?),
        )?))),
        "Autocorrelation" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Autocorrelation::new(p(0)?, p(1)?),
        )?))),
        "HurstExponent" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::HurstExponent::new(p(0)?, p(1)?),
        )?))),
        "RviVolatility" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RviVolatility::new(p(0)?),
        )?))),
        "LaguerreRsi" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::LaguerreRsi::new(float_param(params, 0, kind)?),
        )?))),
        "ConnorsRsi" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::ConnorsRsi::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "SuperSmoother" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::SuperSmoother::new(p(0)?),
        )?))),
        "FisherTransform" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::FisherTransform::new(p(0)?),
        )?))),
        "InverseFisherTransform" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::InverseFisherTransform::new(float_param(params, 0, kind)?),
        )?))),
        "Decycler" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Decycler::new(p(0)?),
        )?))),
        "DecyclerOscillator" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::DecyclerOscillator::new(p(0)?, p(1)?),
        )?))),
        "RoofingFilter" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RoofingFilter::new(p(0)?, p(1)?),
        )?))),
        "CenterOfGravity" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::CenterOfGravity::new(p(0)?),
        )?))),
        "CyberneticCycle" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::CyberneticCycle::new(p(0)?),
        )?))),
        "InstantaneousTrendline" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::InstantaneousTrendline::new(p(0)?),
        )?))),
        "EhlersStochastic" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::EhlersStochastic::new(p(0)?),
        )?))),
        "EmpiricalModeDecomposition" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::EmpiricalModeDecomposition::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "Fama" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Fama::new(float_param(params, 0, kind)?, float_param(params, 1, kind)?),
        )?))),
        "CalmarRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::CalmarRatio::new(p(0)?),
        )?))),
        "MaxDrawdown" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::MaxDrawdown::new(p(0)?),
        )?))),
        "AverageDrawdown" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::AverageDrawdown::new(p(0)?),
        )?))),
        "PainIndex" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::PainIndex::new(p(0)?),
        )?))),
        "ProfitFactor" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::ProfitFactor::new(p(0)?),
        )?))),
        "GainLossRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::GainLossRatio::new(p(0)?),
        )?))),
        "KellyCriterion" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::KellyCriterion::new(p(0)?),
        )?))),
        "SharpeRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::SharpeRatio::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "SortinoRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::SortinoRatio::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "OmegaRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::OmegaRatio::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "ValueAtRisk" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::ValueAtRisk::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "ConditionalValueAtRisk" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::ConditionalValueAtRisk::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "MidPoint" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::MidPoint::new(p(0)?),
        )?))),
        "Rocp" => Ok(Box::new(ScalarClose(map_new(kind, wc::Rocp::new(p(0)?))?))),
        "Rocr" => Ok(Box::new(ScalarClose(map_new(kind, wc::Rocr::new(p(0)?))?))),
        "Rocr100" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Rocr100::new(p(0)?),
        )?))),
        "LinRegIntercept" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::LinRegIntercept::new(p(0)?),
        )?))),
        "Tsf" => Ok(Box::new(ScalarClose(map_new(kind, wc::Tsf::new(p(0)?))?))),
        "LogReturn" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::LogReturn::new(p(0)?),
        )?))),
        "RealizedVolatility" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RealizedVolatility::new(p(0)?),
        )?))),
        "RollingIqr" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RollingIqr::new(p(0)?),
        )?))),
        "RollingPercentileRank" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RollingPercentileRank::new(p(0)?),
        )?))),
        "RollingQuantile" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RollingQuantile::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "TrendLabel" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::TrendLabel::new(p(0)?),
        )?))),
        "JumpIndicator" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::JumpIndicator::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "RegimeLabel" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RegimeLabel::new(p(0)?, p(1)?),
        )?))),
        "WinRate" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::WinRate::new(p(0)?),
        )?))),
        "Expectancy" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Expectancy::new(p(0)?),
        )?))),
        "SineWeightedMa" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::SineWeightedMa::new(p(0)?),
        )?))),
        "GeometricMa" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::GeometricMa::new(p(0)?),
        )?))),
        "Ehma" => Ok(Box::new(ScalarClose(map_new(kind, wc::Ehma::new(p(0)?))?))),
        "MedianMa" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::MedianMa::new(p(0)?),
        )?))),
        "AdaptiveLaguerreFilter" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::AdaptiveLaguerreFilter::new(p(0)?),
        )?))),
        "GeneralizedDema" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::GeneralizedDema::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "HoltWinters" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::HoltWinters::new(float_param(params, 0, kind)?, float_param(params, 1, kind)?),
        )?))),
        "DisparityIndex" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::DisparityIndex::new(p(0)?),
        )?))),
        "FisherRsi" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::FisherRsi::new(p(0)?),
        )?))),
        "Rsx" => Ok(Box::new(ScalarClose(map_new(kind, wc::Rsx::new(p(0)?))?))),
        "DynamicMomentumIndex" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::DynamicMomentumIndex::new(p(0)?),
        )?))),
        "Rmi" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Rmi::new(p(0)?, p(1)?),
        )?))),
        "DerivativeOscillator" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::DerivativeOscillator::new(p(0)?, p(1)?, p(2)?, p(3)?),
        )?))),
        "TrendStrengthIndex" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::TrendStrengthIndex::new(p(0)?),
        )?))),
        "TsfOscillator" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::TsfOscillator::new(p(0)?),
        )?))),
        "MacdHistogram" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::MacdHistogram::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "PpoHistogram" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::PpoHistogram::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "BipowerVariation" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::BipowerVariation::new(p(0)?),
        )?))),
        "EwmaVolatility" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::EwmaVolatility::new(float_param(params, 0, kind)?),
        )?))),
        "Garch11" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Garch11::new(
                float_param(params, 0, kind)?,
                float_param(params, 1, kind)?,
                float_param(params, 2, kind)?,
            ),
        )?))),
        "VolatilityOfVolatility" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::VolatilityOfVolatility::new(p(0)?, p(1)?),
        )?))),
        "JarqueBera" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::JarqueBera::new(p(0)?),
        )?))),
        "RollingMinMaxScaler" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RollingMinMaxScaler::new(p(0)?),
        )?))),
        "ShannonEntropy" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::ShannonEntropy::new(p(0)?, p(1)?),
        )?))),
        "SampleEntropy" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::SampleEntropy::new(p(0)?, p(1)?, float_param(params, 2, kind)?),
        )?))),
        "HighpassFilter" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::HighpassFilter::new(p(0)?),
        )?))),
        "Reflex" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Reflex::new(p(0)?),
        )?))),
        "Trendflex" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Trendflex::new(p(0)?),
        )?))),
        "CorrelationTrendIndicator" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::CorrelationTrendIndicator::new(p(0)?),
        )?))),
        "AdaptiveRsi" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::AdaptiveRsi::new(p(0)?),
        )?))),
        "UniversalOscillator" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::UniversalOscillator::new(p(0)?),
        )?))),
        "BandpassFilter" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::BandpassFilter::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "EvenBetterSinewave" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::EvenBetterSinewave::new(p(0)?, p(1)?),
        )?))),
        "AutocorrelationPeriodogram" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::AutocorrelationPeriodogram::new(p(0)?, p(1)?),
        )?))),
        "SterlingRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::SterlingRatio::new(p(0)?),
        )?))),
        "BurkeRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::BurkeRatio::new(p(0)?),
        )?))),
        "MartinRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::MartinRatio::new(p(0)?),
        )?))),
        "TailRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::TailRatio::new(p(0)?),
        )?))),
        "KRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::KRatio::new(p(0)?),
        )?))),
        "CommonSenseRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::CommonSenseRatio::new(p(0)?),
        )?))),
        "GainToPainRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::GainToPainRatio::new(p(0)?),
        )?))),
        "UpsidePotentialRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::UpsidePotentialRatio::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "M2Measure" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::M2Measure::new(
                p(0)?,
                float_param(params, 1, kind)?,
                float_param(params, 2, kind)?,
            ),
        )?))),
        // --- generated candlestick patterns (Input = Candle) ---
        "Hammer" => Ok(Box::new(CandleIn(wc::Hammer::new()))),
        "InvertedHammer" => Ok(Box::new(CandleIn(wc::InvertedHammer::new()))),
        "HangingMan" => Ok(Box::new(CandleIn(wc::HangingMan::new()))),
        "ShootingStar" => Ok(Box::new(CandleIn(wc::ShootingStar::new()))),
        "Engulfing" => Ok(Box::new(CandleIn(wc::Engulfing::new()))),
        "Harami" => Ok(Box::new(CandleIn(wc::Harami::new()))),
        "MorningEveningStar" => Ok(Box::new(CandleIn(wc::MorningEveningStar::new()))),
        "ThreeSoldiersOrCrows" => Ok(Box::new(CandleIn(wc::ThreeSoldiersOrCrows::new()))),
        "PiercingDarkCloud" => Ok(Box::new(CandleIn(wc::PiercingDarkCloud::new()))),
        "Marubozu" => Ok(Box::new(CandleIn(wc::Marubozu::new()))),
        "Tweezer" => Ok(Box::new(CandleIn(wc::Tweezer::new()))),
        "SpinningTop" => Ok(Box::new(CandleIn(wc::SpinningTop::new()))),
        "ThreeInside" => Ok(Box::new(CandleIn(wc::ThreeInside::new()))),
        "ThreeOutside" => Ok(Box::new(CandleIn(wc::ThreeOutside::new()))),
        "TwoCrows" => Ok(Box::new(CandleIn(wc::TwoCrows::new()))),
        "UpsideGapTwoCrows" => Ok(Box::new(CandleIn(wc::UpsideGapTwoCrows::new()))),
        "IdenticalThreeCrows" => Ok(Box::new(CandleIn(wc::IdenticalThreeCrows::new()))),
        "ThreeLineStrike" => Ok(Box::new(CandleIn(wc::ThreeLineStrike::new()))),
        "ThreeStarsInSouth" => Ok(Box::new(CandleIn(wc::ThreeStarsInSouth::new()))),
        "AbandonedBaby" => Ok(Box::new(CandleIn(wc::AbandonedBaby::new()))),
        "AdvanceBlock" => Ok(Box::new(CandleIn(wc::AdvanceBlock::new()))),
        "BeltHold" => Ok(Box::new(CandleIn(wc::BeltHold::new()))),
        "Breakaway" => Ok(Box::new(CandleIn(wc::Breakaway::new()))),
        "Counterattack" => Ok(Box::new(CandleIn(wc::Counterattack::new()))),
        "DojiStar" => Ok(Box::new(CandleIn(wc::DojiStar::new()))),
        "DragonflyDoji" => Ok(Box::new(CandleIn(wc::DragonflyDoji::new()))),
        "GravestoneDoji" => Ok(Box::new(CandleIn(wc::GravestoneDoji::new()))),
        "LongLeggedDoji" => Ok(Box::new(CandleIn(wc::LongLeggedDoji::new()))),
        "RickshawMan" => Ok(Box::new(CandleIn(wc::RickshawMan::new()))),
        "EveningDojiStar" => Ok(Box::new(CandleIn(wc::EveningDojiStar::new()))),
        "MorningDojiStar" => Ok(Box::new(CandleIn(wc::MorningDojiStar::new()))),
        "GapSideBySideWhite" => Ok(Box::new(CandleIn(wc::GapSideBySideWhite::new()))),
        "HighWave" => Ok(Box::new(CandleIn(wc::HighWave::new()))),
        "Hikkake" => Ok(Box::new(CandleIn(wc::Hikkake::new()))),
        "HikkakeModified" => Ok(Box::new(CandleIn(wc::HikkakeModified::new()))),
        "HomingPigeon" => Ok(Box::new(CandleIn(wc::HomingPigeon::new()))),
        "OnNeck" => Ok(Box::new(CandleIn(wc::OnNeck::new()))),
        "InNeck" => Ok(Box::new(CandleIn(wc::InNeck::new()))),
        "Thrusting" => Ok(Box::new(CandleIn(wc::Thrusting::new()))),
        "SeparatingLines" => Ok(Box::new(CandleIn(wc::SeparatingLines::new()))),
        "Kicking" => Ok(Box::new(CandleIn(wc::Kicking::new()))),
        "KickingByLength" => Ok(Box::new(CandleIn(wc::KickingByLength::new()))),
        "LadderBottom" => Ok(Box::new(CandleIn(wc::LadderBottom::new()))),
        "MatHold" => Ok(Box::new(CandleIn(wc::MatHold::new()))),
        "MatchingLow" => Ok(Box::new(CandleIn(wc::MatchingLow::new()))),
        "LongLine" => Ok(Box::new(CandleIn(wc::LongLine::new()))),
        "ShortLine" => Ok(Box::new(CandleIn(wc::ShortLine::new()))),
        "RisingThreeMethods" => Ok(Box::new(CandleIn(wc::RisingThreeMethods::new()))),
        "FallingThreeMethods" => Ok(Box::new(CandleIn(wc::FallingThreeMethods::new()))),
        "UpsideGapThreeMethods" => Ok(Box::new(CandleIn(wc::UpsideGapThreeMethods::new()))),
        "DownsideGapThreeMethods" => Ok(Box::new(CandleIn(wc::DownsideGapThreeMethods::new()))),
        "StalledPattern" => Ok(Box::new(CandleIn(wc::StalledPattern::new()))),
        "StickSandwich" => Ok(Box::new(CandleIn(wc::StickSandwich::new()))),
        "Takuri" => Ok(Box::new(CandleIn(wc::Takuri::new()))),
        "ClosingMarubozu" => Ok(Box::new(CandleIn(wc::ClosingMarubozu::new()))),
        "OpeningMarubozu" => Ok(Box::new(CandleIn(wc::OpeningMarubozu::new()))),
        "TasukiGap" => Ok(Box::new(CandleIn(wc::TasukiGap::new()))),
        "UniqueThreeRiver" => Ok(Box::new(CandleIn(wc::UniqueThreeRiver::new()))),
        "ConcealingBabySwallow" => Ok(Box::new(CandleIn(wc::ConcealingBabySwallow::new()))),
        "DoubleTopBottom" => Ok(Box::new(CandleIn(wc::DoubleTopBottom::new()))),
        "TripleTopBottom" => Ok(Box::new(CandleIn(wc::TripleTopBottom::new()))),
        "HeadAndShoulders" => Ok(Box::new(CandleIn(wc::HeadAndShoulders::new()))),
        "Triangle" => Ok(Box::new(CandleIn(wc::Triangle::new()))),
        "Wedge" => Ok(Box::new(CandleIn(wc::Wedge::new()))),
        "FlagPennant" => Ok(Box::new(CandleIn(wc::FlagPennant::new()))),
        "RectangleRange" => Ok(Box::new(CandleIn(wc::RectangleRange::new()))),
        "CupAndHandle" => Ok(Box::new(CandleIn(wc::CupAndHandle::new()))),
        "Abcd" => Ok(Box::new(CandleIn(wc::Abcd::new()))),
        "Gartley" => Ok(Box::new(CandleIn(wc::Gartley::new()))),
        "Butterfly" => Ok(Box::new(CandleIn(wc::Butterfly::new()))),
        "Bat" => Ok(Box::new(CandleIn(wc::Bat::new()))),
        "Crab" => Ok(Box::new(CandleIn(wc::Crab::new()))),
        "Shark" => Ok(Box::new(CandleIn(wc::Shark::new()))),
        "Cypher" => Ok(Box::new(CandleIn(wc::Cypher::new()))),
        "ThreeDrives" => Ok(Box::new(CandleIn(wc::ThreeDrives::new()))),
        "TdCamouflage" => Ok(Box::new(CandleIn(wc::TdCamouflage::new()))),
        "TdClop" => Ok(Box::new(CandleIn(wc::TdClop::new()))),
        "TdClopwin" => Ok(Box::new(CandleIn(wc::TdClopwin::new()))),
        "TdPropulsion" => Ok(Box::new(CandleIn(wc::TdPropulsion::new()))),
        "TdTrap" => Ok(Box::new(CandleIn(wc::TdTrap::new()))),
        "Tristar" => Ok(Box::new(CandleIn(wc::Tristar::new()))),
        "HaramiCross" => Ok(Box::new(CandleIn(wc::HaramiCross::new()))),
        "TowerTopBottom" => Ok(Box::new(CandleIn(wc::TowerTopBottom::new()))),
        // --- hand-written candle-input scalar indicators ---
        "Atr" => Ok(Box::new(CandleIn(map_new(kind, wc::Atr::new(p(0)?))?))),
        "Cci" => Ok(Box::new(CandleIn(map_new(kind, wc::Cci::new(p(0)?))?))),
        "WilliamsR" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::WilliamsR::new(p(0)?),
        )?))),
        "Mfi" => Ok(Box::new(CandleIn(map_new(kind, wc::Mfi::new(p(0)?))?))),
        "Vwap" => Ok(Box::new(CandleIn(wc::Vwap::new()))),
        "Obv" => Ok(Box::new(CandleIn(wc::Obv::new()))),
        // --- hand-written multi-output indicators ---
        "Macd" => Ok(Box::new(MacdWrap {
            inner: map_new(kind, wc::MacdIndicator::new(p(0)?, p(1)?, p(2)?))?,
            last: Vec::new(),
        })),
        "Bollinger" => Ok(Box::new(BollingerWrap {
            inner: map_new(
                kind,
                wc::BollingerBands::new(p(0)?, float_param(params, 1, kind)?),
            )?,
            last: Vec::new(),
        })),
        "Stochastic" => Ok(Box::new(StochasticWrap {
            inner: map_new(kind, wc::Stochastic::new(p(0)?, p(1)?))?,
            last: Vec::new(),
        })),
        "Adx" => Ok(Box::new(AdxWrap {
            inner: map_new(kind, wc::Adx::new(p(0)?))?,
            last: Vec::new(),
        })),
        "Aroon" => Ok(Box::new(AroonWrap {
            inner: map_new(kind, wc::Aroon::new(p(0)?))?,
            last: Vec::new(),
        })),
        "Keltner" => Ok(Box::new(KeltnerWrap {
            inner: map_new(
                kind,
                wc::Keltner::new(p(0)?, p(1)?, float_param(params, 2, kind)?),
            )?,
            last: Vec::new(),
        })),
        "Donchian" => Ok(Box::new(DonchianWrap {
            inner: map_new(kind, wc::Donchian::new(p(0)?))?,
            last: Vec::new(),
        })),
        other => Err(BacktestError::UnknownIndicator(other.to_string())),
    }
}

/// Every registered indicator with valid default parameters (235 indicators).
#[cfg(test)]
const ALL_SPECS: &[(&str, &[f64])] = &[
    ("Sma", &[14.0]),
    ("Ema", &[14.0]),
    ("Wma", &[14.0]),
    ("Rsi", &[14.0]),
    ("Dema", &[14.0]),
    ("Tema", &[14.0]),
    ("Hma", &[14.0]),
    ("Roc", &[14.0]),
    ("Trix", &[14.0]),
    ("Smma", &[14.0]),
    ("Trima", &[14.0]),
    ("Zlema", &[14.0]),
    ("T3", &[5.0, 0.7]),
    ("Alma", &[9.0, 0.85, 6.0]),
    ("PolarizedFractalEfficiency", &[10.0, 5.0]),
    ("WavePm", &[3.0, 7.0]),
    ("McGinleyDynamic", &[14.0]),
    ("Frama", &[14.0]),
    ("Vidya", &[3.0, 7.0]),
    ("Jma", &[7.0, 0.0, 2.0]),
    ("Mom", &[14.0]),
    ("Cmo", &[14.0]),
    ("Tsi", &[3.0, 7.0]),
    ("Pmo", &[3.0, 7.0]),
    ("Tii", &[3.0, 7.0]),
    ("StochRsi", &[3.0, 7.0]),
    ("Dpo", &[14.0]),
    ("Ppo", &[3.0, 7.0]),
    ("Apo", &[3.0, 7.0]),
    ("Cfo", &[14.0]),
    ("ElderImpulse", &[3.0, 7.0, 14.0, 28.0]),
    ("Stc", &[10.0, 23.0, 10.0, 0.5]),
    ("Coppock", &[3.0, 7.0, 14.0]),
    ("StdDev", &[14.0]),
    ("UlcerIndex", &[14.0]),
    ("HistoricalVolatility", &[3.0, 7.0]),
    ("BollingerBandwidth", &[14.0, 2.0]),
    ("PercentB", &[14.0, 2.0]),
    ("LinearRegression", &[14.0]),
    ("LinRegSlope", &[14.0]),
    ("VerticalHorizontalFilter", &[14.0]),
    ("ZScore", &[14.0]),
    ("LinRegAngle", &[14.0]),
    ("Variance", &[14.0]),
    ("CoefficientOfVariation", &[14.0]),
    ("Skewness", &[14.0]),
    ("Kurtosis", &[14.0]),
    ("StandardError", &[14.0]),
    ("DetrendedStdDev", &[14.0]),
    ("RSquared", &[14.0]),
    ("MedianAbsoluteDeviation", &[14.0]),
    ("Autocorrelation", &[10.0, 1.0]),
    ("HurstExponent", &[100.0, 4.0]),
    ("RviVolatility", &[14.0]),
    ("LaguerreRsi", &[0.5]),
    ("ConnorsRsi", &[3.0, 7.0, 14.0]),
    ("SuperSmoother", &[14.0]),
    ("FisherTransform", &[14.0]),
    ("InverseFisherTransform", &[2.0]),
    ("Decycler", &[14.0]),
    ("DecyclerOscillator", &[3.0, 7.0]),
    ("RoofingFilter", &[3.0, 7.0]),
    ("CenterOfGravity", &[14.0]),
    ("CyberneticCycle", &[14.0]),
    ("InstantaneousTrendline", &[14.0]),
    ("EhlersStochastic", &[14.0]),
    ("EmpiricalModeDecomposition", &[20.0, 0.1]),
    ("Fama", &[0.5, 0.05]),
    ("CalmarRatio", &[14.0]),
    ("MaxDrawdown", &[14.0]),
    ("AverageDrawdown", &[14.0]),
    ("PainIndex", &[14.0]),
    ("ProfitFactor", &[14.0]),
    ("GainLossRatio", &[14.0]),
    ("KellyCriterion", &[14.0]),
    ("SharpeRatio", &[14.0, 2.0]),
    ("SortinoRatio", &[14.0, 2.0]),
    ("OmegaRatio", &[14.0, 2.0]),
    ("ValueAtRisk", &[20.0, 0.95]),
    ("ConditionalValueAtRisk", &[20.0, 0.95]),
    ("MidPoint", &[14.0]),
    ("Rocp", &[14.0]),
    ("Rocr", &[14.0]),
    ("Rocr100", &[14.0]),
    ("LinRegIntercept", &[14.0]),
    ("Tsf", &[14.0]),
    ("LogReturn", &[14.0]),
    ("RealizedVolatility", &[14.0]),
    ("RollingIqr", &[14.0]),
    ("RollingPercentileRank", &[14.0]),
    ("RollingQuantile", &[20.0, 0.5]),
    ("TrendLabel", &[14.0]),
    ("JumpIndicator", &[14.0, 2.0]),
    ("RegimeLabel", &[3.0, 7.0]),
    ("WinRate", &[14.0]),
    ("Expectancy", &[14.0]),
    ("SineWeightedMa", &[14.0]),
    ("GeometricMa", &[14.0]),
    ("Ehma", &[14.0]),
    ("MedianMa", &[14.0]),
    ("AdaptiveLaguerreFilter", &[20.0]),
    ("GeneralizedDema", &[5.0, 0.7]),
    ("HoltWinters", &[0.5, 0.1]),
    ("DisparityIndex", &[14.0]),
    ("FisherRsi", &[14.0]),
    ("Rsx", &[14.0]),
    ("DynamicMomentumIndex", &[14.0]),
    ("Rmi", &[3.0, 7.0]),
    ("DerivativeOscillator", &[3.0, 7.0, 14.0, 28.0]),
    ("TrendStrengthIndex", &[14.0]),
    ("TsfOscillator", &[14.0]),
    ("MacdHistogram", &[3.0, 7.0, 14.0]),
    ("PpoHistogram", &[3.0, 7.0, 14.0]),
    ("BipowerVariation", &[14.0]),
    ("EwmaVolatility", &[0.94]),
    ("Garch11", &[2e-06, 0.1, 0.88]),
    ("VolatilityOfVolatility", &[3.0, 7.0]),
    ("JarqueBera", &[14.0]),
    ("RollingMinMaxScaler", &[14.0]),
    ("ShannonEntropy", &[3.0, 7.0]),
    ("SampleEntropy", &[20.0, 2.0, 0.2]),
    ("HighpassFilter", &[14.0]),
    ("Reflex", &[14.0]),
    ("Trendflex", &[14.0]),
    ("CorrelationTrendIndicator", &[14.0]),
    ("AdaptiveRsi", &[14.0]),
    ("UniversalOscillator", &[14.0]),
    ("BandpassFilter", &[20.0, 0.3]),
    ("EvenBetterSinewave", &[40.0, 10.0]),
    ("AutocorrelationPeriodogram", &[10.0, 48.0]),
    ("SterlingRatio", &[14.0]),
    ("BurkeRatio", &[14.0]),
    ("MartinRatio", &[14.0]),
    ("TailRatio", &[14.0]),
    ("KRatio", &[14.0]),
    ("CommonSenseRatio", &[14.0]),
    ("GainToPainRatio", &[14.0]),
    ("UpsidePotentialRatio", &[14.0, 2.0]),
    ("M2Measure", &[14.0, 2.0, 0.5]),
    ("Hammer", &[]),
    ("InvertedHammer", &[]),
    ("HangingMan", &[]),
    ("ShootingStar", &[]),
    ("Engulfing", &[]),
    ("Harami", &[]),
    ("MorningEveningStar", &[]),
    ("ThreeSoldiersOrCrows", &[]),
    ("PiercingDarkCloud", &[]),
    ("Marubozu", &[]),
    ("Tweezer", &[]),
    ("SpinningTop", &[]),
    ("ThreeInside", &[]),
    ("ThreeOutside", &[]),
    ("TwoCrows", &[]),
    ("UpsideGapTwoCrows", &[]),
    ("IdenticalThreeCrows", &[]),
    ("ThreeLineStrike", &[]),
    ("ThreeStarsInSouth", &[]),
    ("AbandonedBaby", &[]),
    ("AdvanceBlock", &[]),
    ("BeltHold", &[]),
    ("Breakaway", &[]),
    ("Counterattack", &[]),
    ("DojiStar", &[]),
    ("DragonflyDoji", &[]),
    ("GravestoneDoji", &[]),
    ("LongLeggedDoji", &[]),
    ("RickshawMan", &[]),
    ("EveningDojiStar", &[]),
    ("MorningDojiStar", &[]),
    ("GapSideBySideWhite", &[]),
    ("HighWave", &[]),
    ("Hikkake", &[]),
    ("HikkakeModified", &[]),
    ("HomingPigeon", &[]),
    ("OnNeck", &[]),
    ("InNeck", &[]),
    ("Thrusting", &[]),
    ("SeparatingLines", &[]),
    ("Kicking", &[]),
    ("KickingByLength", &[]),
    ("LadderBottom", &[]),
    ("MatHold", &[]),
    ("MatchingLow", &[]),
    ("LongLine", &[]),
    ("ShortLine", &[]),
    ("RisingThreeMethods", &[]),
    ("FallingThreeMethods", &[]),
    ("UpsideGapThreeMethods", &[]),
    ("DownsideGapThreeMethods", &[]),
    ("StalledPattern", &[]),
    ("StickSandwich", &[]),
    ("Takuri", &[]),
    ("ClosingMarubozu", &[]),
    ("OpeningMarubozu", &[]),
    ("TasukiGap", &[]),
    ("UniqueThreeRiver", &[]),
    ("ConcealingBabySwallow", &[]),
    ("DoubleTopBottom", &[]),
    ("TripleTopBottom", &[]),
    ("HeadAndShoulders", &[]),
    ("Triangle", &[]),
    ("Wedge", &[]),
    ("FlagPennant", &[]),
    ("RectangleRange", &[]),
    ("CupAndHandle", &[]),
    ("Abcd", &[]),
    ("Gartley", &[]),
    ("Butterfly", &[]),
    ("Bat", &[]),
    ("Crab", &[]),
    ("Shark", &[]),
    ("Cypher", &[]),
    ("ThreeDrives", &[]),
    ("TdCamouflage", &[]),
    ("TdClop", &[]),
    ("TdClopwin", &[]),
    ("TdPropulsion", &[]),
    ("TdTrap", &[]),
    ("Tristar", &[]),
    ("HaramiCross", &[]),
    ("TowerTopBottom", &[]),
    ("Atr", &[14.0]),
    ("Cci", &[20.0]),
    ("WilliamsR", &[14.0]),
    ("Mfi", &[14.0]),
    ("Vwap", &[]),
    ("Obv", &[]),
    ("Macd", &[12.0, 26.0, 9.0]),
    ("Bollinger", &[20.0, 2.0]),
    ("Stochastic", &[14.0, 3.0]),
    ("Adx", &[14.0]),
    ("Aroon", &[14.0]),
    ("Keltner", &[20.0, 10.0, 2.0]),
    ("Donchian", &[20.0]),
];

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
        for (kind, params) in ALL_SPECS {
            assert!(build(kind, params).is_ok(), "{kind} should build");
        }
    }

    #[test]
    fn registry_has_full_catalog() {
        // Generated scalar + candlestick families plus the hand-written set.
        assert!(
            ALL_SPECS.len() >= 200,
            "catalog too small: {}",
            ALL_SPECS.len()
        );
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
