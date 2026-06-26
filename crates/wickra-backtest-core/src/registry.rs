//! Indicator registry: constructs `wickra-core` indicators by name and wraps
//! them behind a uniform, object-safe [`EvalIndicator`] the engine can drive
//! from a [`Candle`].
//!
//! GENERATED FILE — do not edit by hand. Regenerate with:
//!
//! ```text
//! python tools/gen_registry.py --wickra ../wickra --out crates/wickra-backtest-core/src/registry.rs
//! cargo fmt --all
//! ```
//!
//! Source of truth: the wickra-core indicator sources (the `Indicator` impls,
//! `new` signatures and Output structs). Every single-instrument indicator
//! (`Input = f64` fed the close, or `Input = Candle`) with a scalar `f64` or
//! all-`f64`-field struct output is registered, plus pairwise
//! (`Input = (f64, f64)`) indicators fed `(close, reference_close)` from the
//! reference series. Multi-output indicators expose named fields, referenced in
//! the spec as `"name.field"`.

use wickra_core::{
    self as wc, Candle as CoreCandle, DerivativesTick as CoreDerivativesTick, Indicator,
    OrderBook as CoreOrderBook, Trade as CoreTrade, TradeQuote as CoreTradeQuote,
};

use crate::data::Candle;
use crate::error::{BacktestError, Result};

/// Everything an indicator may consume on one bar. Single-instrument indicators
/// use `candle`; pairwise indicators also use `reference`; derivatives,
/// order-book and trade indicators use `deriv` / `orderbook` / `trades`. Feeds
/// that are absent are `None` / empty.
pub struct BarInput<'a> {
    /// The current bar.
    pub candle: &'a Candle,
    /// The reference series' close (for pairwise indicators).
    pub reference: Option<f64>,
    /// The derivatives tick for this bar (for derivatives indicators).
    pub deriv: Option<CoreDerivativesTick>,
    /// The order-book snapshot for this bar (for order-book indicators).
    pub orderbook: Option<&'a CoreOrderBook>,
    /// The trades that printed within this bar (for trade-flow indicators),
    /// replayed in order; empty when there is no trade feed.
    pub trades: &'a [CoreTrade],
}

/// A uniform, object-safe indicator the engine drives one bar at a time.
pub trait EvalIndicator: Send {
    /// Feed one bar's [`BarInput`]; returns the primary value, or `None` while
    /// warming up or when the required feed is absent.
    fn update(&mut self, input: &BarInput) -> Option<f64>;
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
    fn update(&mut self, input: &BarInput) -> Option<f64> {
        self.0.update(input.candle.close)
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
    fn update(&mut self, input: &BarInput) -> Option<f64> {
        input.candle.to_core().ok().and_then(|c| self.0.update(c))
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        Vec::new()
    }
    fn warmup(&self) -> usize {
        self.0.warmup_period()
    }
}

/// Wraps a pairwise (`Input = (f64, f64)`) single-output indicator, fed
/// `(close, reference_close)`. Without a reference series it yields `None`.
struct PairClose<I>(I);

impl<I> EvalIndicator for PairClose<I>
where
    I: Indicator<Input = (f64, f64), Output = f64> + Send,
{
    fn update(&mut self, input: &BarInput) -> Option<f64> {
        input
            .reference
            .and_then(|r| self.0.update((input.candle.close, r)))
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        Vec::new()
    }
    fn warmup(&self) -> usize {
        self.0.warmup_period()
    }
}

/// Wraps a derivatives (`Input = DerivativesTick`) single-output indicator.
/// Without a derivatives feed it yields `None`.
struct DerivativesIn<I>(I);

impl<I> EvalIndicator for DerivativesIn<I>
where
    I: Indicator<Input = CoreDerivativesTick, Output = f64> + Send,
{
    fn update(&mut self, input: &BarInput) -> Option<f64> {
        input.deriv.and_then(|d| self.0.update(d))
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        Vec::new()
    }
    fn warmup(&self) -> usize {
        self.0.warmup_period()
    }
}

/// Wraps an order-book (`Input = OrderBook`) single-output indicator. Without an
/// order-book feed it yields `None`.
struct OrderBookIn<I>(I);

impl<I> EvalIndicator for OrderBookIn<I>
where
    I: Indicator<Input = CoreOrderBook, Output = f64> + Send,
{
    fn update(&mut self, input: &BarInput) -> Option<f64> {
        input.orderbook.and_then(|ob| self.0.update(ob.clone()))
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        Vec::new()
    }
    fn warmup(&self) -> usize {
        self.0.warmup_period()
    }
}

/// Wraps a trade (`Input = Trade`) single-output indicator: replays the bar's
/// trades in order, returning the value after the last. With no trades it yields
/// `None`.
struct TradeIn<I>(I);

impl<I> EvalIndicator for TradeIn<I>
where
    I: Indicator<Input = CoreTrade, Output = f64> + Send,
{
    fn update(&mut self, input: &BarInput) -> Option<f64> {
        let mut last = None;
        for &t in input.trades {
            last = self.0.update(t);
        }
        last
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        Vec::new()
    }
    fn warmup(&self) -> usize {
        self.0.warmup_period()
    }
}

/// Wraps a trade-quote (`Input = TradeQuote`) single-output indicator: pairs each
/// bar trade with the prevailing mid (the order book's mid if present, else the
/// bar close) and replays them. With no trades it yields `None`.
struct TradeQuoteIn<I>(I);

impl<I> EvalIndicator for TradeQuoteIn<I>
where
    I: Indicator<Input = CoreTradeQuote, Output = f64> + Send,
{
    fn update(&mut self, input: &BarInput) -> Option<f64> {
        let mid = input
            .orderbook
            .and_then(|ob| match (ob.best_bid(), ob.best_ask()) {
                (Some(bid), Some(ask)) => Some(f64::midpoint(ask.price, bid.price)),
                _ => None,
            })
            .unwrap_or(input.candle.close);
        let mut last = None;
        for &t in input.trades {
            if let Ok(tq) = CoreTradeQuote::new(t, mid) {
                last = self.0.update(tq);
            }
        }
        last
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        Vec::new()
    }
    fn warmup(&self) -> usize {
        self.0.warmup_period()
    }
}

/// Define a multi-output wrapper over an `Input = f64` indicator. The primary
/// value (bare `"name"` reference) is the first field; all fields are exposed
/// for `"name.field"` references.
macro_rules! multi_close {
    ($wrap:ident, $ty:ident, $first:ident, [$($f:ident),+]) => {
        struct $wrap {
            inner: wc::$ty,
            last: Vec<(&'static str, f64)>,
        }
        impl $wrap {
            fn wrap(inner: wc::$ty) -> Self {
                Self { inner, last: Vec::new() }
            }
        }
        impl EvalIndicator for $wrap {
            fn update(&mut self, input: &BarInput) -> Option<f64> {
                let out = self.inner.update(input.candle.close)?;
                self.last = vec![$((stringify!($f), out.$f)),+];
                Some(out.$first)
            }
            fn fields(&self) -> Vec<(&'static str, f64)> {
                self.last.clone()
            }
            fn warmup(&self) -> usize {
                self.inner.warmup_period()
            }
        }
    };
}

/// Define a multi-output wrapper over an `Input = Candle` indicator.
macro_rules! multi_candle {
    ($wrap:ident, $ty:ident, $first:ident, [$($f:ident),+]) => {
        struct $wrap {
            inner: wc::$ty,
            last: Vec<(&'static str, f64)>,
        }
        impl $wrap {
            fn wrap(inner: wc::$ty) -> Self {
                Self { inner, last: Vec::new() }
            }
        }
        impl EvalIndicator for $wrap {
            fn update(&mut self, input: &BarInput) -> Option<f64> {
                let c = input.candle.to_core().ok()?;
                let out = self.inner.update(c)?;
                self.last = vec![$((stringify!($f), out.$f)),+];
                Some(out.$first)
            }
            fn fields(&self) -> Vec<(&'static str, f64)> {
                self.last.clone()
            }
            fn warmup(&self) -> usize {
                self.inner.warmup_period()
            }
        }
    };
}

/// Define a multi-output wrapper over a pairwise (`Input = (f64, f64)`)
/// indicator, fed `(close, reference_close)`. Without a reference it yields none.
macro_rules! multi_pair {
    ($wrap:ident, $ty:ident, $first:ident, [$($f:ident),+]) => {
        struct $wrap {
            inner: wc::$ty,
            last: Vec<(&'static str, f64)>,
        }
        impl $wrap {
            fn wrap(inner: wc::$ty) -> Self {
                Self { inner, last: Vec::new() }
            }
        }
        impl EvalIndicator for $wrap {
            fn update(&mut self, input: &BarInput) -> Option<f64> {
                let out = self.inner.update((input.candle.close, input.reference?))?;
                self.last = vec![$((stringify!($f), out.$f)),+];
                Some(out.$first)
            }
            fn fields(&self) -> Vec<(&'static str, f64)> {
                self.last.clone()
            }
            fn warmup(&self) -> usize {
                self.inner.warmup_period()
            }
        }
    };
}

/// Define a multi-output wrapper over a derivatives (`Input = DerivativesTick`)
/// indicator. Without a derivatives feed it yields none.
macro_rules! multi_deriv {
    ($wrap:ident, $ty:ident, $first:ident, [$($f:ident),+]) => {
        struct $wrap {
            inner: wc::$ty,
            last: Vec<(&'static str, f64)>,
        }
        impl $wrap {
            fn wrap(inner: wc::$ty) -> Self {
                Self { inner, last: Vec::new() }
            }
        }
        impl EvalIndicator for $wrap {
            fn update(&mut self, input: &BarInput) -> Option<f64> {
                let out = self.inner.update(input.deriv?)?;
                self.last = vec![$((stringify!($f), out.$f)),+];
                Some(out.$first)
            }
            fn fields(&self) -> Vec<(&'static str, f64)> {
                self.last.clone()
            }
            fn warmup(&self) -> usize {
                self.inner.warmup_period()
            }
        }
    };
}

multi_candle!(
    AccelerationBandsWrap,
    AccelerationBands,
    upper,
    [upper, middle, lower]
);
multi_candle!(AdxWrap, Adx, plus_di, [plus_di, minus_di, adx]);
multi_candle!(AlligatorWrap, Alligator, jaw, [jaw, teeth, lips]);
multi_candle!(
    AndrewsPitchforkWrap,
    AndrewsPitchfork,
    median,
    [median, upper, lower]
);
multi_candle!(AroonWrap, Aroon, up, [up, down]);
multi_candle!(AtrBandsWrap, AtrBands, upper, [upper, middle, lower]);
multi_candle!(AtrRatchetWrap, AtrRatchet, value, [value, direction]);
multi_candle!(
    AutoFibWrap,
    AutoFib,
    level_0,
    [level_0, level_236, level_382, level_500, level_618, level_786, level_1000]
);
multi_close!(
    BollingerBandsWrap,
    BollingerBands,
    upper,
    [upper, middle, lower, stddev]
);
multi_close!(BomarBandsWrap, BomarBands, upper, [upper, middle, lower]);
multi_candle!(
    CamarillaWrap,
    Camarilla,
    pp,
    [pp, r1, r2, r3, r4, s1, s2, s3, s4]
);
multi_candle!(CandleVolumeWrap, CandleVolume, body, [body, width]);
multi_candle!(
    CentralPivotRangeWrap,
    CentralPivotRange,
    pivot,
    [pivot, tc, bc]
);
multi_candle!(
    ChandeKrollStopWrap,
    ChandeKrollStop,
    stop_long,
    [stop_long, stop_short]
);
multi_candle!(
    ChandelierExitWrap,
    ChandelierExit,
    long_stop,
    [long_stop, short_stop]
);
multi_candle!(
    ClassicPivotsWrap,
    ClassicPivots,
    pp,
    [pp, r1, r2, r3, s1, s2, s3]
);
multi_candle!(CompositeProfileWrap, CompositeProfile, poc, [poc, vah, val]);
multi_candle!(DemarkPivotsWrap, DemarkPivots, pp, [pp, r1, s1]);
multi_candle!(DonchianWrap, Donchian, upper, [upper, middle, lower]);
multi_candle!(
    DonchianStopWrap,
    DonchianStop,
    stop_long,
    [stop_long, stop_short]
);
multi_close!(
    DoubleBollingerWrap,
    DoubleBollinger,
    upper_outer,
    [upper_outer, upper_inner, middle, lower_inner, lower_outer]
);
multi_candle!(ElderRayWrap, ElderRay, bull_power, [bull_power, bear_power]);
multi_candle!(ElderSafeZoneWrap, ElderSafeZone, value, [value, direction]);
multi_candle!(EquivolumeWrap, Equivolume, height, [height, width]);
multi_candle!(FibArcsWrap, FibArcs, arc_382, [arc_382, arc_500, arc_618]);
multi_candle!(
    FibChannelWrap,
    FibChannel,
    base,
    [base, level_618, level_1000, level_1618]
);
multi_candle!(FibConfluenceWrap, FibConfluence, price, [price, strength]);
multi_candle!(
    FibExtensionWrap,
    FibExtension,
    level_1272,
    [level_1272, level_1414, level_1618, level_2000, level_2618]
);
multi_candle!(FibFanWrap, FibFan, fan_382, [fan_382, fan_500, fan_618]);
multi_candle!(
    FibProjectionWrap,
    FibProjection,
    level_618,
    [level_618, level_1000, level_1618, level_2618]
);
multi_candle!(
    FibRetracementWrap,
    FibRetracement,
    level_0,
    [level_0, level_236, level_382, level_500, level_618, level_786, level_1000]
);
multi_candle!(
    FibTimeZonesWrap,
    FibTimeZones,
    on_zone,
    [on_zone, bars_to_next]
);
multi_candle!(
    FibonacciPivotsWrap,
    FibonacciPivots,
    pp,
    [pp, r1, r2, r3, s1, s2, s3]
);
multi_candle!(
    FractalChaosBandsWrap,
    FractalChaosBands,
    upper,
    [upper, lower]
);
multi_candle!(GatorOscillatorWrap, GatorOscillator, upper, [upper, lower]);
multi_candle!(GoldenPocketWrap, GoldenPocket, low, [low, mid, high]);
multi_candle!(HeikinAshiWrap, HeikinAshi, open, [open, high, low, close]);
multi_candle!(HighLowVolumeNodesWrap, HighLowVolumeNodes, hvn, [hvn, lvn]);
multi_close!(HtPhasorWrap, HtPhasor, inphase, [inphase, quadrature]);
multi_candle!(
    HurstChannelWrap,
    HurstChannel,
    upper,
    [upper, middle, lower]
);
multi_candle!(InitialBalanceWrap, InitialBalance, high, [high, low]);
multi_candle!(KaseDevStopWrap, KaseDevStop, value, [value, direction]);
multi_candle!(
    KasePermissionStochasticWrap,
    KasePermissionStochastic,
    fast,
    [fast, slow]
);
multi_candle!(KeltnerWrap, Keltner, upper, [upper, middle, lower]);
multi_close!(KstWrap, Kst, kst, [kst, signal]);
multi_close!(
    LinRegChannelWrap,
    LinRegChannel,
    upper,
    [upper, middle, lower]
);
multi_close!(MaEnvelopeWrap, MaEnvelope, upper, [upper, middle, lower]);
multi_close!(MacdFixWrap, MacdFix, macd, [macd, signal, histogram]);
multi_close!(
    MacdIndicatorWrap,
    MacdIndicator,
    macd,
    [macd, signal, histogram]
);
multi_close!(MamaWrap, Mama, mama, [mama, fama]);
multi_close!(
    MedianChannelWrap,
    MedianChannel,
    upper,
    [upper, middle, lower]
);
multi_candle!(
    ModifiedMaStopWrap,
    ModifiedMaStop,
    value,
    [value, direction]
);
multi_candle!(
    MurreyMathLinesWrap,
    MurreyMathLines,
    mm8_8,
    [mm8_8, mm7_8, mm6_8, mm5_8, mm4_8, mm3_8, mm2_8, mm1_8, mm0_8]
);
multi_candle!(NrtrWrap, Nrtr, value, [value, direction]);
multi_candle!(
    OpeningRangeWrap,
    OpeningRange,
    high,
    [high, low, breakout_distance]
);
multi_candle!(
    OvernightIntradayReturnWrap,
    OvernightIntradayReturn,
    overnight,
    [overnight, intraday]
);
multi_candle!(
    ProjectionBandsWrap,
    ProjectionBands,
    upper,
    [upper, middle, lower]
);
multi_close!(QqeWrap, Qqe, rsi_ma, [rsi_ma, trailing_line]);
multi_close!(
    QuartileBandsWrap,
    QuartileBands,
    upper,
    [upper, middle, lower]
);
multi_candle!(RwiWrap, Rwi, high, [high, low]);
multi_candle!(SessionHighLowWrap, SessionHighLow, high, [high, low]);
multi_candle!(SessionRangeWrap, SessionRange, asia, [asia, eu, us]);
multi_candle!(
    SmoothedHeikinAshiWrap,
    SmoothedHeikinAshi,
    open,
    [open, high, low, close]
);
multi_close!(
    StandardErrorBandsWrap,
    StandardErrorBands,
    upper,
    [upper, middle, lower]
);
multi_candle!(StarcBandsWrap, StarcBands, upper, [upper, middle, lower]);
multi_candle!(StochasticWrap, Stochastic, k, [k, d]);
multi_candle!(SuperTrendWrap, SuperTrend, value, [value, direction]);
multi_candle!(TdLinesWrap, TdLines, resistance, [resistance, support]);
multi_candle!(TdMovingAverageWrap, TdMovingAverage, st1, [st1, st2]);
multi_candle!(TdRangeProjectionWrap, TdRangeProjection, high, [high, low]);
multi_candle!(
    TdRiskLevelWrap,
    TdRiskLevel,
    buy_risk,
    [buy_risk, sell_risk]
);
multi_candle!(
    TdSequentialWrap,
    TdSequential,
    setup,
    [setup, countdown, direction]
);
multi_candle!(
    TpoProfileWrap,
    TpoProfile,
    price_low,
    [price_low, price_high]
);
multi_candle!(TtmSqueezeWrap, TtmSqueeze, squeeze, [squeeze, momentum]);
multi_candle!(ValueAreaWrap, ValueArea, poc, [poc, vah, val]);
multi_candle!(
    VolatilityConeWrap,
    VolatilityCone,
    current,
    [current, min, median, max, percentile]
);
multi_candle!(
    VolumeProfileWrap,
    VolumeProfile,
    price_low,
    [price_low, price_high]
);
multi_candle!(
    VolumeWeightedMacdWrap,
    VolumeWeightedMacd,
    macd,
    [macd, signal, histogram]
);
multi_candle!(
    VolumeWeightedSrWrap,
    VolumeWeightedSr,
    support,
    [support, resistance]
);
multi_candle!(VortexWrap, Vortex, plus, [plus, minus]);
multi_candle!(
    VwapStdDevBandsWrap,
    VwapStdDevBands,
    upper,
    [upper, middle, lower, stddev]
);
multi_candle!(WaveTrendWrap, WaveTrend, wt1, [wt1, wt2]);
multi_candle!(WoodiePivotsWrap, WoodiePivots, pp, [pp, r1, r2, s1, s2]);
multi_close!(
    ZeroLagMacdWrap,
    ZeroLagMacd,
    macd,
    [macd, signal, histogram]
);
multi_candle!(ZigZagWrap, ZigZag, swing, [swing, direction]);
multi_pair!(
    CointegrationWrap,
    Cointegration,
    hedge_ratio,
    [hedge_ratio, spread, adf_stat]
);
multi_pair!(
    KalmanHedgeRatioWrap,
    KalmanHedgeRatio,
    hedge_ratio,
    [hedge_ratio, intercept, spread]
);
multi_pair!(
    LeadLagCrossCorrelationWrap,
    LeadLagCrossCorrelation,
    correlation,
    [correlation]
);
multi_pair!(
    RelativeStrengthABWrap,
    RelativeStrengthAB,
    ratio,
    [ratio, ratio_ma, ratio_rsi]
);
multi_pair!(
    SpreadBollingerBandsWrap,
    SpreadBollingerBands,
    middle,
    [middle, upper, lower, percent_b]
);
multi_deriv!(
    LiquidationFeaturesWrap,
    LiquidationFeatures,
    long,
    [long, short, net, total, imbalance]
);

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

/// Read parameter `idx` as an `i32`.
fn i32_param(params: &[f64], idx: usize, kind: &str) -> Result<i32> {
    let v = float_param(params, idx, kind)?;
    if v.fract().abs() > f64::EPSILON || v > f64::from(i32::MAX) || v < f64::from(i32::MIN) {
        return Err(BacktestError::InvalidParams {
            indicator: kind.to_string(),
            reason: format!("parameter #{idx} must be an i32, got {v}"),
        });
    }
    Ok(v as i32)
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
    match kind {
        // --- scalar single-output (Input = f64), fed the close ---
        "AdaptiveCycle" => Ok(Box::new(ScalarClose(wc::AdaptiveCycle::new()))),
        "AdaptiveLaguerreFilter" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::AdaptiveLaguerreFilter::new(p(0)?),
        )?))),
        "AdaptiveRsi" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::AdaptiveRsi::new(p(0)?),
        )?))),
        "Alma" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Alma::new(
                p(0)?,
                float_param(params, 1, kind)?,
                float_param(params, 2, kind)?,
            ),
        )?))),
        "AnchoredRsi" => Ok(Box::new(ScalarClose(wc::AnchoredRsi::new()))),
        "Apo" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Apo::new(p(0)?, p(1)?),
        )?))),
        "Autocorrelation" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Autocorrelation::new(p(0)?, p(1)?),
        )?))),
        "AutocorrelationPeriodogram" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::AutocorrelationPeriodogram::new(p(0)?, p(1)?),
        )?))),
        "AverageDrawdown" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::AverageDrawdown::new(p(0)?),
        )?))),
        "BandpassFilter" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::BandpassFilter::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "BipowerVariation" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::BipowerVariation::new(p(0)?),
        )?))),
        "BollingerBandwidth" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::BollingerBandwidth::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "BurkeRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::BurkeRatio::new(p(0)?),
        )?))),
        "CalmarRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::CalmarRatio::new(p(0)?),
        )?))),
        "CenterOfGravity" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::CenterOfGravity::new(p(0)?),
        )?))),
        "Cfo" => Ok(Box::new(ScalarClose(map_new(kind, wc::Cfo::new(p(0)?))?))),
        "Cmo" => Ok(Box::new(ScalarClose(map_new(kind, wc::Cmo::new(p(0)?))?))),
        "CoefficientOfVariation" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::CoefficientOfVariation::new(p(0)?),
        )?))),
        "CommonSenseRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::CommonSenseRatio::new(p(0)?),
        )?))),
        "ConditionalValueAtRisk" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::ConditionalValueAtRisk::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "ConnorsRsi" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::ConnorsRsi::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "Coppock" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Coppock::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "CorrelationTrendIndicator" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::CorrelationTrendIndicator::new(p(0)?),
        )?))),
        "CyberneticCycle" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::CyberneticCycle::new(p(0)?),
        )?))),
        "Decycler" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Decycler::new(p(0)?),
        )?))),
        "DecyclerOscillator" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::DecyclerOscillator::new(p(0)?, p(1)?),
        )?))),
        "Dema" => Ok(Box::new(ScalarClose(map_new(kind, wc::Dema::new(p(0)?))?))),
        "DerivativeOscillator" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::DerivativeOscillator::new(p(0)?, p(1)?, p(2)?, p(3)?),
        )?))),
        "DetrendedStdDev" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::DetrendedStdDev::new(p(0)?),
        )?))),
        "DisparityIndex" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::DisparityIndex::new(p(0)?),
        )?))),
        "Dpo" => Ok(Box::new(ScalarClose(map_new(kind, wc::Dpo::new(p(0)?))?))),
        "DynamicMomentumIndex" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::DynamicMomentumIndex::new(p(0)?),
        )?))),
        "EhlersStochastic" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::EhlersStochastic::new(p(0)?),
        )?))),
        "Ehma" => Ok(Box::new(ScalarClose(map_new(kind, wc::Ehma::new(p(0)?))?))),
        "ElderImpulse" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::ElderImpulse::new(p(0)?, p(1)?, p(2)?, p(3)?),
        )?))),
        "Ema" => Ok(Box::new(ScalarClose(map_new(kind, wc::Ema::new(p(0)?))?))),
        "EmpiricalModeDecomposition" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::EmpiricalModeDecomposition::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "EvenBetterSinewave" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::EvenBetterSinewave::new(p(0)?, p(1)?),
        )?))),
        "EwmaVolatility" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::EwmaVolatility::new(float_param(params, 0, kind)?),
        )?))),
        "Expectancy" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Expectancy::new(p(0)?),
        )?))),
        "Fama" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Fama::new(float_param(params, 0, kind)?, float_param(params, 1, kind)?),
        )?))),
        "FisherRsi" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::FisherRsi::new(p(0)?),
        )?))),
        "FisherTransform" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::FisherTransform::new(p(0)?),
        )?))),
        "Frama" => Ok(Box::new(ScalarClose(map_new(kind, wc::Frama::new(p(0)?))?))),
        "GainLossRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::GainLossRatio::new(p(0)?),
        )?))),
        "GainToPainRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::GainToPainRatio::new(p(0)?),
        )?))),
        "Garch11" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Garch11::new(
                float_param(params, 0, kind)?,
                float_param(params, 1, kind)?,
                float_param(params, 2, kind)?,
            ),
        )?))),
        "GeneralizedDema" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::GeneralizedDema::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "GeometricMa" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::GeometricMa::new(p(0)?),
        )?))),
        "HighpassFilter" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::HighpassFilter::new(p(0)?),
        )?))),
        "HilbertDominantCycle" => Ok(Box::new(ScalarClose(wc::HilbertDominantCycle::new()))),
        "HistoricalVolatility" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::HistoricalVolatility::new(p(0)?, p(1)?),
        )?))),
        "Hma" => Ok(Box::new(ScalarClose(map_new(kind, wc::Hma::new(p(0)?))?))),
        "HoltWinters" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::HoltWinters::new(float_param(params, 0, kind)?, float_param(params, 1, kind)?),
        )?))),
        "HtDcPhase" => Ok(Box::new(ScalarClose(wc::HtDcPhase::new()))),
        "HtTrendMode" => Ok(Box::new(ScalarClose(wc::HtTrendMode::new()))),
        "HurstExponent" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::HurstExponent::new(p(0)?, p(1)?),
        )?))),
        "InstantaneousTrendline" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::InstantaneousTrendline::new(p(0)?),
        )?))),
        "InverseFisherTransform" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::InverseFisherTransform::new(float_param(params, 0, kind)?),
        )?))),
        "JarqueBera" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::JarqueBera::new(p(0)?),
        )?))),
        "Jma" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Jma::new(
                p(0)?,
                float_param(params, 1, kind)?,
                u32_param(params, 2, kind)?,
            ),
        )?))),
        "JumpIndicator" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::JumpIndicator::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "KRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::KRatio::new(p(0)?),
        )?))),
        "Kama" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Kama::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "KellyCriterion" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::KellyCriterion::new(p(0)?),
        )?))),
        "Kurtosis" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Kurtosis::new(p(0)?),
        )?))),
        "LaguerreRsi" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::LaguerreRsi::new(float_param(params, 0, kind)?),
        )?))),
        "LinRegAngle" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::LinRegAngle::new(p(0)?),
        )?))),
        "LinRegIntercept" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::LinRegIntercept::new(p(0)?),
        )?))),
        "LinRegSlope" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::LinRegSlope::new(p(0)?),
        )?))),
        "LinearRegression" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::LinearRegression::new(p(0)?),
        )?))),
        "LogReturn" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::LogReturn::new(p(0)?),
        )?))),
        "M2Measure" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::M2Measure::new(
                p(0)?,
                float_param(params, 1, kind)?,
                float_param(params, 2, kind)?,
            ),
        )?))),
        "MacdHistogram" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::MacdHistogram::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "MartinRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::MartinRatio::new(p(0)?),
        )?))),
        "MaxDrawdown" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::MaxDrawdown::new(p(0)?),
        )?))),
        "McGinleyDynamic" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::McGinleyDynamic::new(p(0)?),
        )?))),
        "MedianAbsoluteDeviation" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::MedianAbsoluteDeviation::new(p(0)?),
        )?))),
        "MedianMa" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::MedianMa::new(p(0)?),
        )?))),
        "MidPoint" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::MidPoint::new(p(0)?),
        )?))),
        "Mom" => Ok(Box::new(ScalarClose(map_new(kind, wc::Mom::new(p(0)?))?))),
        "OmegaRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::OmegaRatio::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "PainIndex" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::PainIndex::new(p(0)?),
        )?))),
        "PercentB" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::PercentB::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "PercentageTrailingStop" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::PercentageTrailingStop::new(float_param(params, 0, kind)?),
        )?))),
        "Pmo" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Pmo::new(p(0)?, p(1)?),
        )?))),
        "PolarizedFractalEfficiency" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::PolarizedFractalEfficiency::new(p(0)?, p(1)?),
        )?))),
        "Ppo" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Ppo::new(p(0)?, p(1)?),
        )?))),
        "PpoHistogram" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::PpoHistogram::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "ProfitFactor" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::ProfitFactor::new(p(0)?),
        )?))),
        "RSquared" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RSquared::new(p(0)?),
        )?))),
        "RealizedVolatility" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RealizedVolatility::new(p(0)?),
        )?))),
        "RecoveryFactor" => Ok(Box::new(ScalarClose(wc::RecoveryFactor::new()))),
        "Reflex" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Reflex::new(p(0)?),
        )?))),
        "RegimeLabel" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RegimeLabel::new(p(0)?, p(1)?),
        )?))),
        "RenkoTrailingStop" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RenkoTrailingStop::new(float_param(params, 0, kind)?),
        )?))),
        "Rmi" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Rmi::new(p(0)?, p(1)?),
        )?))),
        "Roc" => Ok(Box::new(ScalarClose(map_new(kind, wc::Roc::new(p(0)?))?))),
        "Rocp" => Ok(Box::new(ScalarClose(map_new(kind, wc::Rocp::new(p(0)?))?))),
        "Rocr" => Ok(Box::new(ScalarClose(map_new(kind, wc::Rocr::new(p(0)?))?))),
        "Rocr100" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Rocr100::new(p(0)?),
        )?))),
        "RollingIqr" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RollingIqr::new(p(0)?),
        )?))),
        "RollingMinMaxScaler" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RollingMinMaxScaler::new(p(0)?),
        )?))),
        "RollingPercentileRank" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RollingPercentileRank::new(p(0)?),
        )?))),
        "RollingQuantile" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RollingQuantile::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "RoofingFilter" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RoofingFilter::new(p(0)?, p(1)?),
        )?))),
        "Rsi" => Ok(Box::new(ScalarClose(map_new(kind, wc::Rsi::new(p(0)?))?))),
        "Rsx" => Ok(Box::new(ScalarClose(map_new(kind, wc::Rsx::new(p(0)?))?))),
        "RviVolatility" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::RviVolatility::new(p(0)?),
        )?))),
        "SampleEntropy" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::SampleEntropy::new(p(0)?, p(1)?, float_param(params, 2, kind)?),
        )?))),
        "ShannonEntropy" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::ShannonEntropy::new(p(0)?, p(1)?),
        )?))),
        "SharpeRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::SharpeRatio::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "SineWave" => Ok(Box::new(ScalarClose(wc::SineWave::new()))),
        "SineWeightedMa" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::SineWeightedMa::new(p(0)?),
        )?))),
        "Skewness" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Skewness::new(p(0)?),
        )?))),
        "Sma" => Ok(Box::new(ScalarClose(map_new(kind, wc::Sma::new(p(0)?))?))),
        "Smma" => Ok(Box::new(ScalarClose(map_new(kind, wc::Smma::new(p(0)?))?))),
        "SortinoRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::SortinoRatio::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "StandardError" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::StandardError::new(p(0)?),
        )?))),
        "Stc" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Stc::new(p(0)?, p(1)?, p(2)?, float_param(params, 3, kind)?),
        )?))),
        "StdDev" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::StdDev::new(p(0)?),
        )?))),
        "StepTrailingStop" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::StepTrailingStop::new(float_param(params, 0, kind)?),
        )?))),
        "SterlingRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::SterlingRatio::new(p(0)?),
        )?))),
        "StochRsi" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::StochRsi::new(p(0)?, p(1)?),
        )?))),
        "SuperSmoother" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::SuperSmoother::new(p(0)?),
        )?))),
        "T3" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::T3::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "TailRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::TailRatio::new(p(0)?),
        )?))),
        "Tema" => Ok(Box::new(ScalarClose(map_new(kind, wc::Tema::new(p(0)?))?))),
        "Tii" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Tii::new(p(0)?, p(1)?),
        )?))),
        "TrendLabel" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::TrendLabel::new(p(0)?),
        )?))),
        "TrendStrengthIndex" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::TrendStrengthIndex::new(p(0)?),
        )?))),
        "Trendflex" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Trendflex::new(p(0)?),
        )?))),
        "Trima" => Ok(Box::new(ScalarClose(map_new(kind, wc::Trima::new(p(0)?))?))),
        "Trix" => Ok(Box::new(ScalarClose(map_new(kind, wc::Trix::new(p(0)?))?))),
        "Tsf" => Ok(Box::new(ScalarClose(map_new(kind, wc::Tsf::new(p(0)?))?))),
        "TsfOscillator" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::TsfOscillator::new(p(0)?),
        )?))),
        "Tsi" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Tsi::new(p(0)?, p(1)?),
        )?))),
        "UlcerIndex" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::UlcerIndex::new(p(0)?),
        )?))),
        "UniversalOscillator" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::UniversalOscillator::new(p(0)?),
        )?))),
        "UpsidePotentialRatio" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::UpsidePotentialRatio::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "ValueAtRisk" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::ValueAtRisk::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "Variance" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Variance::new(p(0)?),
        )?))),
        "VerticalHorizontalFilter" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::VerticalHorizontalFilter::new(p(0)?),
        )?))),
        "Vidya" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::Vidya::new(p(0)?, p(1)?),
        )?))),
        "VolatilityOfVolatility" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::VolatilityOfVolatility::new(p(0)?, p(1)?),
        )?))),
        "WavePm" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::WavePm::new(p(0)?, p(1)?),
        )?))),
        "WinRate" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::WinRate::new(p(0)?),
        )?))),
        "Wma" => Ok(Box::new(ScalarClose(map_new(kind, wc::Wma::new(p(0)?))?))),
        "ZScore" => Ok(Box::new(ScalarClose(map_new(
            kind,
            wc::ZScore::new(p(0)?),
        )?))),
        "Zlema" => Ok(Box::new(ScalarClose(map_new(kind, wc::Zlema::new(p(0)?))?))),
        // --- scalar single-output (Input = Candle) ---
        "AbandonedBaby" => Ok(Box::new(CandleIn(wc::AbandonedBaby::new()))),
        "Abcd" => Ok(Box::new(CandleIn(wc::Abcd::new()))),
        "AcceleratorOscillator" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::AcceleratorOscillator::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "AdOscillator" => Ok(Box::new(CandleIn(wc::AdOscillator::new()))),
        "AdaptiveCci" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::AdaptiveCci::new(p(0)?),
        )?))),
        "Adl" => Ok(Box::new(CandleIn(wc::Adl::new()))),
        "AdvanceBlock" => Ok(Box::new(CandleIn(wc::AdvanceBlock::new()))),
        "Adxr" => Ok(Box::new(CandleIn(map_new(kind, wc::Adxr::new(p(0)?))?))),
        "AnchoredVwap" => Ok(Box::new(CandleIn(wc::AnchoredVwap::new()))),
        "AroonOscillator" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::AroonOscillator::new(p(0)?),
        )?))),
        "Atr" => Ok(Box::new(CandleIn(map_new(kind, wc::Atr::new(p(0)?))?))),
        "AtrTrailingStop" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::AtrTrailingStop::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "AverageDailyRange" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::AverageDailyRange::new(p(0)?, i32_param(params, 1, kind)?),
        )?))),
        "AvgPrice" => Ok(Box::new(CandleIn(wc::AvgPrice::new()))),
        "AwesomeOscillator" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::AwesomeOscillator::new(p(0)?, p(1)?),
        )?))),
        "AwesomeOscillatorHistogram" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::AwesomeOscillatorHistogram::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "BalanceOfPower" => Ok(Box::new(CandleIn(wc::BalanceOfPower::new()))),
        "Bat" => Ok(Box::new(CandleIn(wc::Bat::new()))),
        "BeltHold" => Ok(Box::new(CandleIn(wc::BeltHold::new()))),
        "BetterVolume" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::BetterVolume::new(p(0)?),
        )?))),
        "BodySizePct" => Ok(Box::new(CandleIn(wc::BodySizePct::new()))),
        "Breakaway" => Ok(Box::new(CandleIn(wc::Breakaway::new()))),
        "Butterfly" => Ok(Box::new(CandleIn(wc::Butterfly::new()))),
        "Cci" => Ok(Box::new(CandleIn(map_new(kind, wc::Cci::new(p(0)?))?))),
        "ChaikinMoneyFlow" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::ChaikinMoneyFlow::new(p(0)?),
        )?))),
        "ChaikinOscillator" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::ChaikinOscillator::new(p(0)?, p(1)?),
        )?))),
        "ChaikinVolatility" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::ChaikinVolatility::new(p(0)?, p(1)?),
        )?))),
        "ChoppinessIndex" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::ChoppinessIndex::new(p(0)?),
        )?))),
        "CloseVsOpen" => Ok(Box::new(CandleIn(wc::CloseVsOpen::new()))),
        "ClosingMarubozu" => Ok(Box::new(CandleIn(wc::ClosingMarubozu::new()))),
        "ConcealingBabySwallow" => Ok(Box::new(CandleIn(wc::ConcealingBabySwallow::new()))),
        "Counterattack" => Ok(Box::new(CandleIn(wc::Counterattack::new()))),
        "Crab" => Ok(Box::new(CandleIn(wc::Crab::new()))),
        "CupAndHandle" => Ok(Box::new(CandleIn(wc::CupAndHandle::new()))),
        "Cypher" => Ok(Box::new(CandleIn(wc::Cypher::new()))),
        "DemandIndex" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::DemandIndex::new(p(0)?),
        )?))),
        "Doji" => Ok(Box::new(CandleIn(wc::Doji::new()))),
        "DojiStar" => Ok(Box::new(CandleIn(wc::DojiStar::new()))),
        "DoubleTopBottom" => Ok(Box::new(CandleIn(wc::DoubleTopBottom::new()))),
        "DownsideGapThreeMethods" => Ok(Box::new(CandleIn(wc::DownsideGapThreeMethods::new()))),
        "DragonflyDoji" => Ok(Box::new(CandleIn(wc::DragonflyDoji::new()))),
        "DumplingTop" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::DumplingTop::new(p(0)?),
        )?))),
        "Dx" => Ok(Box::new(CandleIn(map_new(kind, wc::Dx::new(p(0)?))?))),
        "EaseOfMovement" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::EaseOfMovement::new(p(0)?),
        )?))),
        "Engulfing" => Ok(Box::new(CandleIn(wc::Engulfing::new()))),
        "EveningDojiStar" => Ok(Box::new(CandleIn(wc::EveningDojiStar::new()))),
        "Evwma" => Ok(Box::new(CandleIn(map_new(kind, wc::Evwma::new(p(0)?))?))),
        "FallingThreeMethods" => Ok(Box::new(CandleIn(wc::FallingThreeMethods::new()))),
        "FlagPennant" => Ok(Box::new(CandleIn(wc::FlagPennant::new()))),
        "ForceIndex" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::ForceIndex::new(p(0)?),
        )?))),
        "FryPanBottom" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::FryPanBottom::new(p(0)?),
        )?))),
        "GapSideBySideWhite" => Ok(Box::new(CandleIn(wc::GapSideBySideWhite::new()))),
        "GarmanKlassVolatility" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::GarmanKlassVolatility::new(p(0)?, p(1)?),
        )?))),
        "Gartley" => Ok(Box::new(CandleIn(wc::Gartley::new()))),
        "GravestoneDoji" => Ok(Box::new(CandleIn(wc::GravestoneDoji::new()))),
        "Hammer" => Ok(Box::new(CandleIn(wc::Hammer::new()))),
        "HangingMan" => Ok(Box::new(CandleIn(wc::HangingMan::new()))),
        "Harami" => Ok(Box::new(CandleIn(wc::Harami::new()))),
        "HaramiCross" => Ok(Box::new(CandleIn(wc::HaramiCross::new()))),
        "HeadAndShoulders" => Ok(Box::new(CandleIn(wc::HeadAndShoulders::new()))),
        "HeikinAshiOscillator" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::HeikinAshiOscillator::new(p(0)?),
        )?))),
        "HiLoActivator" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::HiLoActivator::new(p(0)?),
        )?))),
        "HighLowRange" => Ok(Box::new(CandleIn(wc::HighLowRange::new()))),
        "HighWave" => Ok(Box::new(CandleIn(wc::HighWave::new()))),
        "Hikkake" => Ok(Box::new(CandleIn(wc::Hikkake::new()))),
        "HikkakeModified" => Ok(Box::new(CandleIn(wc::HikkakeModified::new()))),
        "HomingPigeon" => Ok(Box::new(CandleIn(wc::HomingPigeon::new()))),
        "IdenticalThreeCrows" => Ok(Box::new(CandleIn(wc::IdenticalThreeCrows::new()))),
        "InNeck" => Ok(Box::new(CandleIn(wc::InNeck::new()))),
        "Inertia" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::Inertia::new(p(0)?, p(1)?),
        )?))),
        "IntradayIntensity" => Ok(Box::new(CandleIn(wc::IntradayIntensity::new()))),
        "IntradayMomentumIndex" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::IntradayMomentumIndex::new(p(0)?),
        )?))),
        "InvertedHammer" => Ok(Box::new(CandleIn(wc::InvertedHammer::new()))),
        "Kicking" => Ok(Box::new(CandleIn(wc::Kicking::new()))),
        "KickingByLength" => Ok(Box::new(CandleIn(wc::KickingByLength::new()))),
        "Kvo" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::Kvo::new(p(0)?, p(1)?),
        )?))),
        "LadderBottom" => Ok(Box::new(CandleIn(wc::LadderBottom::new()))),
        "LongLeggedDoji" => Ok(Box::new(CandleIn(wc::LongLeggedDoji::new()))),
        "LongLine" => Ok(Box::new(CandleIn(wc::LongLine::new()))),
        "MarketFacilitationIndex" => Ok(Box::new(CandleIn(wc::MarketFacilitationIndex::new()))),
        "Marubozu" => Ok(Box::new(CandleIn(wc::Marubozu::new()))),
        "MassIndex" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::MassIndex::new(p(0)?, p(1)?),
        )?))),
        "MatHold" => Ok(Box::new(CandleIn(wc::MatHold::new()))),
        "MatchingLow" => Ok(Box::new(CandleIn(wc::MatchingLow::new()))),
        "MedianPrice" => Ok(Box::new(CandleIn(wc::MedianPrice::new()))),
        "Mfi" => Ok(Box::new(CandleIn(map_new(kind, wc::Mfi::new(p(0)?))?))),
        "MidPrice" => Ok(Box::new(CandleIn(map_new(kind, wc::MidPrice::new(p(0)?))?))),
        "MinusDi" => Ok(Box::new(CandleIn(map_new(kind, wc::MinusDi::new(p(0)?))?))),
        "MinusDm" => Ok(Box::new(CandleIn(map_new(kind, wc::MinusDm::new(p(0)?))?))),
        "MorningDojiStar" => Ok(Box::new(CandleIn(wc::MorningDojiStar::new()))),
        "MorningEveningStar" => Ok(Box::new(CandleIn(wc::MorningEveningStar::new()))),
        "NakedPoc" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::NakedPoc::new(p(0)?, p(1)?),
        )?))),
        "Natr" => Ok(Box::new(CandleIn(map_new(kind, wc::Natr::new(p(0)?))?))),
        "NewPriceLines" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::NewPriceLines::new(p(0)?),
        )?))),
        "Nvi" => Ok(Box::new(CandleIn(wc::Nvi::new()))),
        "Obv" => Ok(Box::new(CandleIn(wc::Obv::new()))),
        "OnNeck" => Ok(Box::new(CandleIn(wc::OnNeck::new()))),
        "OpeningMarubozu" => Ok(Box::new(CandleIn(wc::OpeningMarubozu::new()))),
        "OvernightGap" => Ok(Box::new(CandleIn(wc::OvernightGap::new(i32_param(
            params, 0, kind,
        )?)))),
        "ParkinsonVolatility" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::ParkinsonVolatility::new(p(0)?, p(1)?),
        )?))),
        "Pgo" => Ok(Box::new(CandleIn(map_new(kind, wc::Pgo::new(p(0)?))?))),
        "PiercingDarkCloud" => Ok(Box::new(CandleIn(wc::PiercingDarkCloud::new()))),
        "PivotReversal" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::PivotReversal::new(p(0)?, p(1)?),
        )?))),
        "PlusDi" => Ok(Box::new(CandleIn(map_new(kind, wc::PlusDi::new(p(0)?))?))),
        "PlusDm" => Ok(Box::new(CandleIn(map_new(kind, wc::PlusDm::new(p(0)?))?))),
        "ProfileShape" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::ProfileShape::new(p(0)?, p(1)?),
        )?))),
        "ProjectionOscillator" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::ProjectionOscillator::new(p(0)?),
        )?))),
        "Psar" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::Psar::new(
                float_param(params, 0, kind)?,
                float_param(params, 1, kind)?,
                float_param(params, 2, kind)?,
            ),
        )?))),
        "Pvi" => Ok(Box::new(CandleIn(wc::Pvi::new()))),
        "Qstick" => Ok(Box::new(CandleIn(map_new(kind, wc::Qstick::new(p(0)?))?))),
        "RectangleRange" => Ok(Box::new(CandleIn(wc::RectangleRange::new()))),
        "RickshawMan" => Ok(Box::new(CandleIn(wc::RickshawMan::new()))),
        "RisingThreeMethods" => Ok(Box::new(CandleIn(wc::RisingThreeMethods::new()))),
        "RogersSatchellVolatility" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::RogersSatchellVolatility::new(p(0)?, p(1)?),
        )?))),
        "RollingVwap" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::RollingVwap::new(p(0)?),
        )?))),
        "Rvi" => Ok(Box::new(CandleIn(map_new(kind, wc::Rvi::new(p(0)?))?))),
        "SarExt" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::SarExt::new(
                float_param(params, 0, kind)?,
                float_param(params, 1, kind)?,
                float_param(params, 2, kind)?,
                float_param(params, 3, kind)?,
                float_param(params, 4, kind)?,
                float_param(params, 5, kind)?,
                float_param(params, 6, kind)?,
                float_param(params, 7, kind)?,
            ),
        )?))),
        "SeasonalZScore" => Ok(Box::new(CandleIn(wc::SeasonalZScore::new(i32_param(
            params, 0, kind,
        )?)))),
        "SeparatingLines" => Ok(Box::new(CandleIn(wc::SeparatingLines::new()))),
        "SessionVwap" => Ok(Box::new(CandleIn(wc::SessionVwap::new(i32_param(
            params, 0, kind,
        )?)))),
        "Shark" => Ok(Box::new(CandleIn(wc::Shark::new()))),
        "ShootingStar" => Ok(Box::new(CandleIn(wc::ShootingStar::new()))),
        "ShortLine" => Ok(Box::new(CandleIn(wc::ShortLine::new()))),
        "SinglePrints" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::SinglePrints::new(p(0)?, p(1)?),
        )?))),
        "Smi" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::Smi::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "SpinningTop" => Ok(Box::new(CandleIn(wc::SpinningTop::new()))),
        "StalledPattern" => Ok(Box::new(CandleIn(wc::StalledPattern::new()))),
        "StickSandwich" => Ok(Box::new(CandleIn(wc::StickSandwich::new()))),
        "StochasticCci" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::StochasticCci::new(p(0)?),
        )?))),
        "Takuri" => Ok(Box::new(CandleIn(wc::Takuri::new()))),
        "TasukiGap" => Ok(Box::new(CandleIn(wc::TasukiGap::new()))),
        "TdCamouflage" => Ok(Box::new(CandleIn(wc::TdCamouflage::new()))),
        "TdClop" => Ok(Box::new(CandleIn(wc::TdClop::new()))),
        "TdClopwin" => Ok(Box::new(CandleIn(wc::TdClopwin::new()))),
        "TdCombo" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::TdCombo::new(p(0)?, p(1)?, p(2)?, p(3)?),
        )?))),
        "TdCountdown" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::TdCountdown::new(p(0)?, p(1)?, p(2)?, p(3)?),
        )?))),
        "TdDWave" => Ok(Box::new(CandleIn(map_new(kind, wc::TdDWave::new(p(0)?))?))),
        "TdDeMarker" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::TdDeMarker::new(p(0)?),
        )?))),
        "TdDifferential" => Ok(Box::new(CandleIn(wc::TdDifferential::new()))),
        "TdOpen" => Ok(Box::new(CandleIn(wc::TdOpen::new()))),
        "TdPressure" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::TdPressure::new(p(0)?),
        )?))),
        "TdPropulsion" => Ok(Box::new(CandleIn(wc::TdPropulsion::new()))),
        "TdRei" => Ok(Box::new(CandleIn(map_new(kind, wc::TdRei::new(p(0)?))?))),
        "TdSetup" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::TdSetup::new(p(0)?, p(1)?),
        )?))),
        "TdTrap" => Ok(Box::new(CandleIn(wc::TdTrap::new()))),
        "ThreeDrives" => Ok(Box::new(CandleIn(wc::ThreeDrives::new()))),
        "ThreeInside" => Ok(Box::new(CandleIn(wc::ThreeInside::new()))),
        "ThreeLineBreak" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::ThreeLineBreak::new(p(0)?),
        )?))),
        "ThreeLineStrike" => Ok(Box::new(CandleIn(wc::ThreeLineStrike::new()))),
        "ThreeOutside" => Ok(Box::new(CandleIn(wc::ThreeOutside::new()))),
        "ThreeSoldiersOrCrows" => Ok(Box::new(CandleIn(wc::ThreeSoldiersOrCrows::new()))),
        "ThreeStarsInSouth" => Ok(Box::new(CandleIn(wc::ThreeStarsInSouth::new()))),
        "Thrusting" => Ok(Box::new(CandleIn(wc::Thrusting::new()))),
        "TimeBasedStop" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::TimeBasedStop::new(p(0)?),
        )?))),
        "TowerTopBottom" => Ok(Box::new(CandleIn(wc::TowerTopBottom::new()))),
        "TradeVolumeIndex" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::TradeVolumeIndex::new(float_param(params, 0, kind)?),
        )?))),
        "Triangle" => Ok(Box::new(CandleIn(wc::Triangle::new()))),
        "TripleTopBottom" => Ok(Box::new(CandleIn(wc::TripleTopBottom::new()))),
        "Tristar" => Ok(Box::new(CandleIn(wc::Tristar::new()))),
        "TrueRange" => Ok(Box::new(CandleIn(wc::TrueRange::new()))),
        "Tsv" => Ok(Box::new(CandleIn(map_new(kind, wc::Tsv::new(p(0)?))?))),
        "TtmTrend" => Ok(Box::new(CandleIn(map_new(kind, wc::TtmTrend::new(p(0)?))?))),
        "TurnOfMonth" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::TurnOfMonth::new(
                u32_param(params, 0, kind)?,
                u32_param(params, 1, kind)?,
                i32_param(params, 2, kind)?,
            ),
        )?))),
        "Tweezer" => Ok(Box::new(CandleIn(wc::Tweezer::new()))),
        "TwiggsMoneyFlow" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::TwiggsMoneyFlow::new(p(0)?),
        )?))),
        "TwoCrows" => Ok(Box::new(CandleIn(wc::TwoCrows::new()))),
        "TypicalPrice" => Ok(Box::new(CandleIn(wc::TypicalPrice::new()))),
        "UltimateOscillator" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::UltimateOscillator::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "UniqueThreeRiver" => Ok(Box::new(CandleIn(wc::UniqueThreeRiver::new()))),
        "UpsideGapThreeMethods" => Ok(Box::new(CandleIn(wc::UpsideGapThreeMethods::new()))),
        "UpsideGapTwoCrows" => Ok(Box::new(CandleIn(wc::UpsideGapTwoCrows::new()))),
        "VolatilityRatio" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::VolatilityRatio::new(p(0)?),
        )?))),
        "VoltyStop" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::VoltyStop::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "VolumeOscillator" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::VolumeOscillator::new(p(0)?, p(1)?),
        )?))),
        "VolumePriceTrend" => Ok(Box::new(CandleIn(wc::VolumePriceTrend::new()))),
        "VolumeRsi" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::VolumeRsi::new(p(0)?),
        )?))),
        "Vwap" => Ok(Box::new(CandleIn(wc::Vwap::new()))),
        "Vwma" => Ok(Box::new(CandleIn(map_new(kind, wc::Vwma::new(p(0)?))?))),
        "Vzo" => Ok(Box::new(CandleIn(map_new(kind, wc::Vzo::new(p(0)?))?))),
        "Wad" => Ok(Box::new(CandleIn(wc::Wad::new()))),
        "Wedge" => Ok(Box::new(CandleIn(wc::Wedge::new()))),
        "WeightedClose" => Ok(Box::new(CandleIn(wc::WeightedClose::new()))),
        "WickRatio" => Ok(Box::new(CandleIn(wc::WickRatio::new()))),
        "WilliamsR" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::WilliamsR::new(p(0)?),
        )?))),
        "YangZhangVolatility" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::YangZhangVolatility::new(p(0)?, p(1)?),
        )?))),
        "YoyoExit" => Ok(Box::new(CandleIn(map_new(
            kind,
            wc::YoyoExit::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        // --- multi-output indicators (named fields) ---
        "AccelerationBands" => Ok(Box::new(AccelerationBandsWrap::wrap(map_new(
            kind,
            wc::AccelerationBands::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "Adx" => Ok(Box::new(AdxWrap::wrap(map_new(kind, wc::Adx::new(p(0)?))?))),
        "Alligator" => Ok(Box::new(AlligatorWrap::wrap(map_new(
            kind,
            wc::Alligator::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "AndrewsPitchfork" => Ok(Box::new(AndrewsPitchforkWrap::wrap(map_new(
            kind,
            wc::AndrewsPitchfork::new(p(0)?),
        )?))),
        "Aroon" => Ok(Box::new(AroonWrap::wrap(map_new(
            kind,
            wc::Aroon::new(p(0)?),
        )?))),
        "AtrBands" => Ok(Box::new(AtrBandsWrap::wrap(map_new(
            kind,
            wc::AtrBands::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "AtrRatchet" => Ok(Box::new(AtrRatchetWrap::wrap(map_new(
            kind,
            wc::AtrRatchet::new(
                p(0)?,
                float_param(params, 1, kind)?,
                float_param(params, 2, kind)?,
            ),
        )?))),
        "AutoFib" => Ok(Box::new(AutoFibWrap::wrap(wc::AutoFib::new()))),
        "BollingerBands" => Ok(Box::new(BollingerBandsWrap::wrap(map_new(
            kind,
            wc::BollingerBands::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "BomarBands" => Ok(Box::new(BomarBandsWrap::wrap(map_new(
            kind,
            wc::BomarBands::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "Camarilla" => Ok(Box::new(CamarillaWrap::wrap(wc::Camarilla::new()))),
        "CandleVolume" => Ok(Box::new(CandleVolumeWrap::wrap(map_new(
            kind,
            wc::CandleVolume::new(p(0)?),
        )?))),
        "CentralPivotRange" => Ok(Box::new(CentralPivotRangeWrap::wrap(
            wc::CentralPivotRange::new(),
        ))),
        "ChandeKrollStop" => Ok(Box::new(ChandeKrollStopWrap::wrap(map_new(
            kind,
            wc::ChandeKrollStop::new(p(0)?, float_param(params, 1, kind)?, p(2)?),
        )?))),
        "ChandelierExit" => Ok(Box::new(ChandelierExitWrap::wrap(map_new(
            kind,
            wc::ChandelierExit::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "ClassicPivots" => Ok(Box::new(ClassicPivotsWrap::wrap(wc::ClassicPivots::new()))),
        "CompositeProfile" => Ok(Box::new(CompositeProfileWrap::wrap(map_new(
            kind,
            wc::CompositeProfile::new(p(0)?, p(1)?, float_param(params, 2, kind)?),
        )?))),
        "DemarkPivots" => Ok(Box::new(DemarkPivotsWrap::wrap(wc::DemarkPivots::new()))),
        "Donchian" => Ok(Box::new(DonchianWrap::wrap(map_new(
            kind,
            wc::Donchian::new(p(0)?),
        )?))),
        "DonchianStop" => Ok(Box::new(DonchianStopWrap::wrap(map_new(
            kind,
            wc::DonchianStop::new(p(0)?),
        )?))),
        "DoubleBollinger" => Ok(Box::new(DoubleBollingerWrap::wrap(map_new(
            kind,
            wc::DoubleBollinger::new(
                p(0)?,
                float_param(params, 1, kind)?,
                float_param(params, 2, kind)?,
            ),
        )?))),
        "ElderRay" => Ok(Box::new(ElderRayWrap::wrap(map_new(
            kind,
            wc::ElderRay::new(p(0)?),
        )?))),
        "ElderSafeZone" => Ok(Box::new(ElderSafeZoneWrap::wrap(map_new(
            kind,
            wc::ElderSafeZone::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "Equivolume" => Ok(Box::new(EquivolumeWrap::wrap(map_new(
            kind,
            wc::Equivolume::new(p(0)?),
        )?))),
        "FibArcs" => Ok(Box::new(FibArcsWrap::wrap(wc::FibArcs::new()))),
        "FibChannel" => Ok(Box::new(FibChannelWrap::wrap(wc::FibChannel::new()))),
        "FibConfluence" => Ok(Box::new(FibConfluenceWrap::wrap(wc::FibConfluence::new()))),
        "FibExtension" => Ok(Box::new(FibExtensionWrap::wrap(wc::FibExtension::new()))),
        "FibFan" => Ok(Box::new(FibFanWrap::wrap(wc::FibFan::new()))),
        "FibProjection" => Ok(Box::new(FibProjectionWrap::wrap(wc::FibProjection::new()))),
        "FibRetracement" => Ok(Box::new(
            FibRetracementWrap::wrap(wc::FibRetracement::new()),
        )),
        "FibTimeZones" => Ok(Box::new(FibTimeZonesWrap::wrap(wc::FibTimeZones::new()))),
        "FibonacciPivots" => Ok(Box::new(FibonacciPivotsWrap::wrap(
            wc::FibonacciPivots::new(),
        ))),
        "FractalChaosBands" => Ok(Box::new(FractalChaosBandsWrap::wrap(map_new(
            kind,
            wc::FractalChaosBands::new(p(0)?),
        )?))),
        "GatorOscillator" => Ok(Box::new(GatorOscillatorWrap::wrap(map_new(
            kind,
            wc::GatorOscillator::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "GoldenPocket" => Ok(Box::new(GoldenPocketWrap::wrap(wc::GoldenPocket::new()))),
        "HeikinAshi" => Ok(Box::new(HeikinAshiWrap::wrap(wc::HeikinAshi::new()))),
        "HighLowVolumeNodes" => Ok(Box::new(HighLowVolumeNodesWrap::wrap(map_new(
            kind,
            wc::HighLowVolumeNodes::new(p(0)?, p(1)?),
        )?))),
        "HtPhasor" => Ok(Box::new(HtPhasorWrap::wrap(wc::HtPhasor::new()))),
        "HurstChannel" => Ok(Box::new(HurstChannelWrap::wrap(map_new(
            kind,
            wc::HurstChannel::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "InitialBalance" => Ok(Box::new(InitialBalanceWrap::wrap(map_new(
            kind,
            wc::InitialBalance::new(p(0)?),
        )?))),
        "KaseDevStop" => Ok(Box::new(KaseDevStopWrap::wrap(map_new(
            kind,
            wc::KaseDevStop::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "KasePermissionStochastic" => Ok(Box::new(KasePermissionStochasticWrap::wrap(map_new(
            kind,
            wc::KasePermissionStochastic::new(p(0)?, p(1)?),
        )?))),
        "Keltner" => Ok(Box::new(KeltnerWrap::wrap(map_new(
            kind,
            wc::Keltner::new(p(0)?, p(1)?, float_param(params, 2, kind)?),
        )?))),
        "Kst" => Ok(Box::new(KstWrap::wrap(map_new(
            kind,
            wc::Kst::new(
                p(0)?,
                p(1)?,
                p(2)?,
                p(3)?,
                p(4)?,
                p(5)?,
                p(6)?,
                p(7)?,
                p(8)?,
            ),
        )?))),
        "LinRegChannel" => Ok(Box::new(LinRegChannelWrap::wrap(map_new(
            kind,
            wc::LinRegChannel::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "MaEnvelope" => Ok(Box::new(MaEnvelopeWrap::wrap(map_new(
            kind,
            wc::MaEnvelope::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "MacdFix" => Ok(Box::new(MacdFixWrap::wrap(map_new(
            kind,
            wc::MacdFix::new(p(0)?),
        )?))),
        "MacdIndicator" => Ok(Box::new(MacdIndicatorWrap::wrap(map_new(
            kind,
            wc::MacdIndicator::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "Mama" => Ok(Box::new(MamaWrap::wrap(map_new(
            kind,
            wc::Mama::new(float_param(params, 0, kind)?, float_param(params, 1, kind)?),
        )?))),
        "MedianChannel" => Ok(Box::new(MedianChannelWrap::wrap(map_new(
            kind,
            wc::MedianChannel::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "ModifiedMaStop" => Ok(Box::new(ModifiedMaStopWrap::wrap(map_new(
            kind,
            wc::ModifiedMaStop::new(p(0)?),
        )?))),
        "MurreyMathLines" => Ok(Box::new(MurreyMathLinesWrap::wrap(map_new(
            kind,
            wc::MurreyMathLines::new(p(0)?),
        )?))),
        "Nrtr" => Ok(Box::new(NrtrWrap::wrap(map_new(
            kind,
            wc::Nrtr::new(float_param(params, 0, kind)?),
        )?))),
        "OpeningRange" => Ok(Box::new(OpeningRangeWrap::wrap(map_new(
            kind,
            wc::OpeningRange::new(p(0)?),
        )?))),
        "OvernightIntradayReturn" => Ok(Box::new(OvernightIntradayReturnWrap::wrap(
            wc::OvernightIntradayReturn::new(i32_param(params, 0, kind)?),
        ))),
        "ProjectionBands" => Ok(Box::new(ProjectionBandsWrap::wrap(map_new(
            kind,
            wc::ProjectionBands::new(p(0)?),
        )?))),
        "Qqe" => Ok(Box::new(QqeWrap::wrap(map_new(
            kind,
            wc::Qqe::new(p(0)?, p(1)?, float_param(params, 2, kind)?),
        )?))),
        "QuartileBands" => Ok(Box::new(QuartileBandsWrap::wrap(map_new(
            kind,
            wc::QuartileBands::new(p(0)?),
        )?))),
        "Rwi" => Ok(Box::new(RwiWrap::wrap(map_new(kind, wc::Rwi::new(p(0)?))?))),
        "SessionHighLow" => Ok(Box::new(SessionHighLowWrap::wrap(wc::SessionHighLow::new(
            i32_param(params, 0, kind)?,
        )))),
        "SessionRange" => Ok(Box::new(SessionRangeWrap::wrap(wc::SessionRange::new(
            i32_param(params, 0, kind)?,
        )))),
        "SmoothedHeikinAshi" => Ok(Box::new(SmoothedHeikinAshiWrap::wrap(map_new(
            kind,
            wc::SmoothedHeikinAshi::new(p(0)?),
        )?))),
        "StandardErrorBands" => Ok(Box::new(StandardErrorBandsWrap::wrap(map_new(
            kind,
            wc::StandardErrorBands::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "StarcBands" => Ok(Box::new(StarcBandsWrap::wrap(map_new(
            kind,
            wc::StarcBands::new(p(0)?, p(1)?, float_param(params, 2, kind)?),
        )?))),
        "Stochastic" => Ok(Box::new(StochasticWrap::wrap(map_new(
            kind,
            wc::Stochastic::new(p(0)?, p(1)?),
        )?))),
        "SuperTrend" => Ok(Box::new(SuperTrendWrap::wrap(map_new(
            kind,
            wc::SuperTrend::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "TdLines" => Ok(Box::new(TdLinesWrap::wrap(map_new(
            kind,
            wc::TdLines::new(p(0)?, p(1)?),
        )?))),
        "TdMovingAverage" => Ok(Box::new(TdMovingAverageWrap::wrap(map_new(
            kind,
            wc::TdMovingAverage::new(p(0)?, p(1)?),
        )?))),
        "TdRangeProjection" => Ok(Box::new(TdRangeProjectionWrap::wrap(
            wc::TdRangeProjection::new(),
        ))),
        "TdRiskLevel" => Ok(Box::new(TdRiskLevelWrap::wrap(map_new(
            kind,
            wc::TdRiskLevel::new(p(0)?, p(1)?),
        )?))),
        "TdSequential" => Ok(Box::new(TdSequentialWrap::wrap(map_new(
            kind,
            wc::TdSequential::new(p(0)?, p(1)?, p(2)?, p(3)?),
        )?))),
        "TpoProfile" => Ok(Box::new(TpoProfileWrap::wrap(map_new(
            kind,
            wc::TpoProfile::new(p(0)?, p(1)?),
        )?))),
        "TtmSqueeze" => Ok(Box::new(TtmSqueezeWrap::wrap(map_new(
            kind,
            wc::TtmSqueeze::new(
                p(0)?,
                float_param(params, 1, kind)?,
                float_param(params, 2, kind)?,
            ),
        )?))),
        "ValueArea" => Ok(Box::new(ValueAreaWrap::wrap(map_new(
            kind,
            wc::ValueArea::new(p(0)?, p(1)?, float_param(params, 2, kind)?),
        )?))),
        "VolatilityCone" => Ok(Box::new(VolatilityConeWrap::wrap(map_new(
            kind,
            wc::VolatilityCone::new(p(0)?, p(1)?),
        )?))),
        "VolumeProfile" => Ok(Box::new(VolumeProfileWrap::wrap(map_new(
            kind,
            wc::VolumeProfile::new(p(0)?, p(1)?),
        )?))),
        "VolumeWeightedMacd" => Ok(Box::new(VolumeWeightedMacdWrap::wrap(map_new(
            kind,
            wc::VolumeWeightedMacd::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "VolumeWeightedSr" => Ok(Box::new(VolumeWeightedSrWrap::wrap(map_new(
            kind,
            wc::VolumeWeightedSr::new(p(0)?),
        )?))),
        "Vortex" => Ok(Box::new(VortexWrap::wrap(map_new(
            kind,
            wc::Vortex::new(p(0)?),
        )?))),
        "VwapStdDevBands" => Ok(Box::new(VwapStdDevBandsWrap::wrap(map_new(
            kind,
            wc::VwapStdDevBands::new(float_param(params, 0, kind)?),
        )?))),
        "WaveTrend" => Ok(Box::new(WaveTrendWrap::wrap(map_new(
            kind,
            wc::WaveTrend::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "WoodiePivots" => Ok(Box::new(WoodiePivotsWrap::wrap(wc::WoodiePivots::new()))),
        "ZeroLagMacd" => Ok(Box::new(ZeroLagMacdWrap::wrap(map_new(
            kind,
            wc::ZeroLagMacd::new(p(0)?, p(1)?, p(2)?),
        )?))),
        "ZigZag" => Ok(Box::new(ZigZagWrap::wrap(map_new(
            kind,
            wc::ZigZag::new(float_param(params, 0, kind)?),
        )?))),
        // --- pairwise indicators, fed (close, reference_close) ---
        "Alpha" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::Alpha::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "Beta" => Ok(Box::new(PairClose(map_new(kind, wc::Beta::new(p(0)?))?))),
        "BetaNeutralSpread" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::BetaNeutralSpread::new(p(0)?),
        )?))),
        "DistanceSsd" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::DistanceSsd::new(p(0)?),
        )?))),
        "GrangerCausality" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::GrangerCausality::new(p(0)?, p(1)?),
        )?))),
        "HasbrouckInformationShare" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::HasbrouckInformationShare::new(p(0)?),
        )?))),
        "InformationRatio" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::InformationRatio::new(p(0)?),
        )?))),
        "KendallTau" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::KendallTau::new(p(0)?),
        )?))),
        "OuHalfLife" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::OuHalfLife::new(p(0)?),
        )?))),
        "PairSpreadZScore" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::PairSpreadZScore::new(p(0)?, p(1)?),
        )?))),
        "PairwiseBeta" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::PairwiseBeta::new(p(0)?),
        )?))),
        "PearsonCorrelation" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::PearsonCorrelation::new(p(0)?),
        )?))),
        "RollingCorrelation" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::RollingCorrelation::new(p(0)?),
        )?))),
        "RollingCovariance" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::RollingCovariance::new(p(0)?),
        )?))),
        "SpearmanCorrelation" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::SpearmanCorrelation::new(p(0)?),
        )?))),
        "SpreadAr1Coefficient" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::SpreadAr1Coefficient::new(p(0)?),
        )?))),
        "SpreadHurst" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::SpreadHurst::new(p(0)?),
        )?))),
        "TreynorRatio" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::TreynorRatio::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        "VarianceRatio" => Ok(Box::new(PairClose(map_new(
            kind,
            wc::VarianceRatio::new(p(0)?, p(1)?),
        )?))),
        // --- pairwise multi-output indicators ---
        "Cointegration" => Ok(Box::new(CointegrationWrap::wrap(map_new(
            kind,
            wc::Cointegration::new(p(0)?, p(1)?),
        )?))),
        "KalmanHedgeRatio" => Ok(Box::new(KalmanHedgeRatioWrap::wrap(map_new(
            kind,
            wc::KalmanHedgeRatio::new(float_param(params, 0, kind)?, float_param(params, 1, kind)?),
        )?))),
        "LeadLagCrossCorrelation" => Ok(Box::new(LeadLagCrossCorrelationWrap::wrap(map_new(
            kind,
            wc::LeadLagCrossCorrelation::new(p(0)?, p(1)?),
        )?))),
        "RelativeStrengthAB" => Ok(Box::new(RelativeStrengthABWrap::wrap(map_new(
            kind,
            wc::RelativeStrengthAB::new(p(0)?, p(1)?),
        )?))),
        "SpreadBollingerBands" => Ok(Box::new(SpreadBollingerBandsWrap::wrap(map_new(
            kind,
            wc::SpreadBollingerBands::new(p(0)?, float_param(params, 1, kind)?),
        )?))),
        // --- derivatives indicators, fed the bar's DerivativesTick ---
        "CalendarSpread" => Ok(Box::new(DerivativesIn(wc::CalendarSpread::new()))),
        "EstimatedLeverageRatio" => Ok(Box::new(DerivativesIn(wc::EstimatedLeverageRatio::new()))),
        "FundingBasis" => Ok(Box::new(DerivativesIn(wc::FundingBasis::new()))),
        "FundingImpliedApr" => Ok(Box::new(DerivativesIn(map_new(
            kind,
            wc::FundingImpliedApr::new(float_param(params, 0, kind)?),
        )?))),
        "FundingRate" => Ok(Box::new(DerivativesIn(wc::FundingRate::new()))),
        "FundingRateMean" => Ok(Box::new(DerivativesIn(map_new(
            kind,
            wc::FundingRateMean::new(p(0)?),
        )?))),
        "FundingRateZScore" => Ok(Box::new(DerivativesIn(map_new(
            kind,
            wc::FundingRateZScore::new(p(0)?),
        )?))),
        "LongShortRatio" => Ok(Box::new(DerivativesIn(wc::LongShortRatio::new()))),
        "OIPriceDivergence" => Ok(Box::new(DerivativesIn(map_new(
            kind,
            wc::OIPriceDivergence::new(p(0)?),
        )?))),
        "OIWeighted" => Ok(Box::new(DerivativesIn(wc::OIWeighted::new()))),
        "OiToVolumeRatio" => Ok(Box::new(DerivativesIn(wc::OiToVolumeRatio::new()))),
        "OpenInterestDelta" => Ok(Box::new(DerivativesIn(wc::OpenInterestDelta::new()))),
        "OpenInterestMomentum" => Ok(Box::new(DerivativesIn(map_new(
            kind,
            wc::OpenInterestMomentum::new(p(0)?),
        )?))),
        "PerpetualPremiumIndex" => Ok(Box::new(DerivativesIn(wc::PerpetualPremiumIndex::new()))),
        "TakerBuySellRatio" => Ok(Box::new(DerivativesIn(wc::TakerBuySellRatio::new()))),
        "TermStructureBasis" => Ok(Box::new(DerivativesIn(wc::TermStructureBasis::new()))),
        "LiquidationFeatures" => Ok(Box::new(LiquidationFeaturesWrap::wrap(
            wc::LiquidationFeatures::new(),
        ))),
        // --- order-book indicators, fed the bar's OrderBook ---
        "DepthSlope" => Ok(Box::new(OrderBookIn(wc::DepthSlope::new()))),
        "Microprice" => Ok(Box::new(OrderBookIn(wc::Microprice::new()))),
        "OrderBookImbalanceFull" => Ok(Box::new(OrderBookIn(wc::OrderBookImbalanceFull::new()))),
        "OrderBookImbalanceTop1" => Ok(Box::new(OrderBookIn(wc::OrderBookImbalanceTop1::new()))),
        "OrderBookImbalanceTopN" => Ok(Box::new(OrderBookIn(map_new(
            kind,
            wc::OrderBookImbalanceTopN::new(p(0)?),
        )?))),
        "OrderFlowImbalance" => Ok(Box::new(OrderBookIn(map_new(
            kind,
            wc::OrderFlowImbalance::new(p(0)?),
        )?))),
        "QuotedSpread" => Ok(Box::new(OrderBookIn(wc::QuotedSpread::new()))),
        // --- trade-flow indicators, fed the bar's trades ---
        "AmihudIlliquidity" => Ok(Box::new(TradeIn(map_new(
            kind,
            wc::AmihudIlliquidity::new(p(0)?),
        )?))),
        "CumulativeVolumeDelta" => Ok(Box::new(TradeIn(wc::CumulativeVolumeDelta::new()))),
        "Pin" => Ok(Box::new(TradeIn(map_new(kind, wc::Pin::new(p(0)?))?))),
        "RollMeasure" => Ok(Box::new(TradeIn(map_new(
            kind,
            wc::RollMeasure::new(p(0)?),
        )?))),
        "SignedVolume" => Ok(Box::new(TradeIn(wc::SignedVolume::new()))),
        "TradeImbalance" => Ok(Box::new(TradeIn(map_new(
            kind,
            wc::TradeImbalance::new(p(0)?),
        )?))),
        "TradeSignAutocorrelation" => Ok(Box::new(TradeIn(map_new(
            kind,
            wc::TradeSignAutocorrelation::new(p(0)?),
        )?))),
        "Vpin" => Ok(Box::new(TradeIn(map_new(
            kind,
            wc::Vpin::new(float_param(params, 0, kind)?, p(1)?),
        )?))),
        // --- trade-quote indicators, fed trades + the mid ---
        "EffectiveSpread" => Ok(Box::new(TradeQuoteIn(wc::EffectiveSpread::new()))),
        "KylesLambda" => Ok(Box::new(TradeQuoteIn(map_new(
            kind,
            wc::KylesLambda::new(p(0)?),
        )?))),
        "RealizedSpread" => Ok(Box::new(TradeQuoteIn(map_new(
            kind,
            wc::RealizedSpread::new(p(0)?),
        )?))),
        // --- friendly aliases ---
        "Macd" => build("MacdIndicator", params),
        "Bollinger" => build("BollingerBands", params),
        other => Err(BacktestError::UnknownIndicator(other.to_string())),
    }
}

/// Every registered indicator with valid default parameters (480 indicators).
#[cfg(test)]
const ALL_SPECS: &[(&str, &[f64])] = &[
    ("AdaptiveCycle", &[]),
    ("AdaptiveLaguerreFilter", &[20.0]),
    ("AdaptiveRsi", &[14.0]),
    ("Alma", &[9.0, 0.85, 6.0]),
    ("AnchoredRsi", &[]),
    ("Apo", &[3.0, 7.0]),
    ("Autocorrelation", &[10.0, 1.0]),
    ("AutocorrelationPeriodogram", &[10.0, 48.0]),
    ("AverageDrawdown", &[14.0]),
    ("BandpassFilter", &[20.0, 0.3]),
    ("BipowerVariation", &[14.0]),
    ("BollingerBandwidth", &[14.0, 2.0]),
    ("BurkeRatio", &[14.0]),
    ("CalmarRatio", &[14.0]),
    ("CenterOfGravity", &[14.0]),
    ("Cfo", &[14.0]),
    ("Cmo", &[14.0]),
    ("CoefficientOfVariation", &[14.0]),
    ("CommonSenseRatio", &[14.0]),
    ("ConditionalValueAtRisk", &[20.0, 0.95]),
    ("ConnorsRsi", &[3.0, 7.0, 14.0]),
    ("Coppock", &[3.0, 7.0, 14.0]),
    ("CorrelationTrendIndicator", &[14.0]),
    ("CyberneticCycle", &[14.0]),
    ("Decycler", &[14.0]),
    ("DecyclerOscillator", &[3.0, 7.0]),
    ("Dema", &[14.0]),
    ("DerivativeOscillator", &[3.0, 7.0, 14.0, 28.0]),
    ("DetrendedStdDev", &[14.0]),
    ("DisparityIndex", &[14.0]),
    ("Dpo", &[14.0]),
    ("DynamicMomentumIndex", &[14.0]),
    ("EhlersStochastic", &[14.0]),
    ("Ehma", &[14.0]),
    ("ElderImpulse", &[3.0, 7.0, 14.0, 28.0]),
    ("Ema", &[14.0]),
    ("EmpiricalModeDecomposition", &[20.0, 0.1]),
    ("EvenBetterSinewave", &[40.0, 10.0]),
    ("EwmaVolatility", &[0.94]),
    ("Expectancy", &[14.0]),
    ("Fama", &[0.5, 0.05]),
    ("FisherRsi", &[14.0]),
    ("FisherTransform", &[14.0]),
    ("Frama", &[14.0]),
    ("GainLossRatio", &[14.0]),
    ("GainToPainRatio", &[14.0]),
    ("Garch11", &[2e-06, 0.1, 0.88]),
    ("GeneralizedDema", &[5.0, 0.7]),
    ("GeometricMa", &[14.0]),
    ("HighpassFilter", &[14.0]),
    ("HilbertDominantCycle", &[]),
    ("HistoricalVolatility", &[3.0, 7.0]),
    ("Hma", &[14.0]),
    ("HoltWinters", &[0.5, 0.1]),
    ("HtDcPhase", &[]),
    ("HtTrendMode", &[]),
    ("HurstExponent", &[100.0, 4.0]),
    ("InstantaneousTrendline", &[14.0]),
    ("InverseFisherTransform", &[2.0]),
    ("JarqueBera", &[14.0]),
    ("Jma", &[7.0, 0.0, 2.0]),
    ("JumpIndicator", &[14.0, 2.0]),
    ("KRatio", &[14.0]),
    ("Kama", &[3.0, 7.0, 14.0]),
    ("KellyCriterion", &[14.0]),
    ("Kurtosis", &[14.0]),
    ("LaguerreRsi", &[0.5]),
    ("LinRegAngle", &[14.0]),
    ("LinRegIntercept", &[14.0]),
    ("LinRegSlope", &[14.0]),
    ("LinearRegression", &[14.0]),
    ("LogReturn", &[14.0]),
    ("M2Measure", &[14.0, 2.0, 0.5]),
    ("MacdHistogram", &[3.0, 7.0, 14.0]),
    ("MartinRatio", &[14.0]),
    ("MaxDrawdown", &[14.0]),
    ("McGinleyDynamic", &[14.0]),
    ("MedianAbsoluteDeviation", &[14.0]),
    ("MedianMa", &[14.0]),
    ("MidPoint", &[14.0]),
    ("Mom", &[14.0]),
    ("OmegaRatio", &[14.0, 2.0]),
    ("PainIndex", &[14.0]),
    ("PercentB", &[14.0, 2.0]),
    ("PercentageTrailingStop", &[2.0]),
    ("Pmo", &[3.0, 7.0]),
    ("PolarizedFractalEfficiency", &[10.0, 5.0]),
    ("Ppo", &[3.0, 7.0]),
    ("PpoHistogram", &[3.0, 7.0, 14.0]),
    ("ProfitFactor", &[14.0]),
    ("RSquared", &[14.0]),
    ("RealizedVolatility", &[14.0]),
    ("RecoveryFactor", &[]),
    ("Reflex", &[14.0]),
    ("RegimeLabel", &[3.0, 7.0]),
    ("RenkoTrailingStop", &[2.0]),
    ("Rmi", &[3.0, 7.0]),
    ("Roc", &[14.0]),
    ("Rocp", &[14.0]),
    ("Rocr", &[14.0]),
    ("Rocr100", &[14.0]),
    ("RollingIqr", &[14.0]),
    ("RollingMinMaxScaler", &[14.0]),
    ("RollingPercentileRank", &[14.0]),
    ("RollingQuantile", &[20.0, 0.5]),
    ("RoofingFilter", &[3.0, 7.0]),
    ("Rsi", &[14.0]),
    ("Rsx", &[14.0]),
    ("RviVolatility", &[14.0]),
    ("SampleEntropy", &[20.0, 2.0, 0.2]),
    ("ShannonEntropy", &[3.0, 7.0]),
    ("SharpeRatio", &[14.0, 2.0]),
    ("SineWave", &[]),
    ("SineWeightedMa", &[14.0]),
    ("Skewness", &[14.0]),
    ("Sma", &[14.0]),
    ("Smma", &[14.0]),
    ("SortinoRatio", &[14.0, 2.0]),
    ("StandardError", &[14.0]),
    ("Stc", &[10.0, 23.0, 10.0, 0.5]),
    ("StdDev", &[14.0]),
    ("StepTrailingStop", &[2.0]),
    ("SterlingRatio", &[14.0]),
    ("StochRsi", &[3.0, 7.0]),
    ("SuperSmoother", &[14.0]),
    ("T3", &[5.0, 0.7]),
    ("TailRatio", &[14.0]),
    ("Tema", &[14.0]),
    ("Tii", &[3.0, 7.0]),
    ("TrendLabel", &[14.0]),
    ("TrendStrengthIndex", &[14.0]),
    ("Trendflex", &[14.0]),
    ("Trima", &[14.0]),
    ("Trix", &[14.0]),
    ("Tsf", &[14.0]),
    ("TsfOscillator", &[14.0]),
    ("Tsi", &[3.0, 7.0]),
    ("UlcerIndex", &[14.0]),
    ("UniversalOscillator", &[14.0]),
    ("UpsidePotentialRatio", &[14.0, 2.0]),
    ("ValueAtRisk", &[20.0, 0.95]),
    ("Variance", &[14.0]),
    ("VerticalHorizontalFilter", &[14.0]),
    ("Vidya", &[3.0, 7.0]),
    ("VolatilityOfVolatility", &[3.0, 7.0]),
    ("WavePm", &[3.0, 7.0]),
    ("WinRate", &[14.0]),
    ("Wma", &[14.0]),
    ("ZScore", &[14.0]),
    ("Zlema", &[14.0]),
    ("AbandonedBaby", &[]),
    ("Abcd", &[]),
    ("AcceleratorOscillator", &[3.0, 7.0, 14.0]),
    ("AdOscillator", &[]),
    ("AdaptiveCci", &[14.0]),
    ("Adl", &[]),
    ("AdvanceBlock", &[]),
    ("Adxr", &[14.0]),
    ("AnchoredVwap", &[]),
    ("AroonOscillator", &[14.0]),
    ("Atr", &[14.0]),
    ("AtrTrailingStop", &[14.0, 2.0]),
    ("AverageDailyRange", &[14.0, 0.0]),
    ("AvgPrice", &[]),
    ("AwesomeOscillator", &[3.0, 7.0]),
    ("AwesomeOscillatorHistogram", &[3.0, 7.0, 14.0]),
    ("BalanceOfPower", &[]),
    ("Bat", &[]),
    ("BeltHold", &[]),
    ("BetterVolume", &[14.0]),
    ("BodySizePct", &[]),
    ("Breakaway", &[]),
    ("Butterfly", &[]),
    ("Cci", &[14.0]),
    ("ChaikinMoneyFlow", &[20.0]),
    ("ChaikinOscillator", &[3.0, 7.0]),
    ("ChaikinVolatility", &[3.0, 7.0]),
    ("ChoppinessIndex", &[14.0]),
    ("CloseVsOpen", &[]),
    ("ClosingMarubozu", &[]),
    ("ConcealingBabySwallow", &[]),
    ("Counterattack", &[]),
    ("Crab", &[]),
    ("CupAndHandle", &[]),
    ("Cypher", &[]),
    ("DemandIndex", &[14.0]),
    ("Doji", &[]),
    ("DojiStar", &[]),
    ("DoubleTopBottom", &[]),
    ("DownsideGapThreeMethods", &[]),
    ("DragonflyDoji", &[]),
    ("DumplingTop", &[14.0]),
    ("Dx", &[14.0]),
    ("EaseOfMovement", &[14.0]),
    ("Engulfing", &[]),
    ("EveningDojiStar", &[]),
    ("Evwma", &[14.0]),
    ("FallingThreeMethods", &[]),
    ("FlagPennant", &[]),
    ("ForceIndex", &[14.0]),
    ("FryPanBottom", &[14.0]),
    ("GapSideBySideWhite", &[]),
    ("GarmanKlassVolatility", &[20.0, 252.0]),
    ("Gartley", &[]),
    ("GravestoneDoji", &[]),
    ("Hammer", &[]),
    ("HangingMan", &[]),
    ("Harami", &[]),
    ("HaramiCross", &[]),
    ("HeadAndShoulders", &[]),
    ("HeikinAshiOscillator", &[14.0]),
    ("HiLoActivator", &[14.0]),
    ("HighLowRange", &[]),
    ("HighWave", &[]),
    ("Hikkake", &[]),
    ("HikkakeModified", &[]),
    ("HomingPigeon", &[]),
    ("IdenticalThreeCrows", &[]),
    ("InNeck", &[]),
    ("Inertia", &[3.0, 7.0]),
    ("IntradayIntensity", &[]),
    ("IntradayMomentumIndex", &[14.0]),
    ("InvertedHammer", &[]),
    ("Kicking", &[]),
    ("KickingByLength", &[]),
    ("Kvo", &[3.0, 7.0]),
    ("LadderBottom", &[]),
    ("LongLeggedDoji", &[]),
    ("LongLine", &[]),
    ("MarketFacilitationIndex", &[]),
    ("Marubozu", &[]),
    ("MassIndex", &[3.0, 7.0]),
    ("MatHold", &[]),
    ("MatchingLow", &[]),
    ("MedianPrice", &[]),
    ("Mfi", &[14.0]),
    ("MidPrice", &[14.0]),
    ("MinusDi", &[14.0]),
    ("MinusDm", &[14.0]),
    ("MorningDojiStar", &[]),
    ("MorningEveningStar", &[]),
    ("NakedPoc", &[3.0, 7.0]),
    ("Natr", &[14.0]),
    ("NewPriceLines", &[14.0]),
    ("Nvi", &[]),
    ("Obv", &[]),
    ("OnNeck", &[]),
    ("OpeningMarubozu", &[]),
    ("OvernightGap", &[0.0]),
    ("ParkinsonVolatility", &[20.0, 252.0]),
    ("Pgo", &[14.0]),
    ("PiercingDarkCloud", &[]),
    ("PivotReversal", &[3.0, 7.0]),
    ("PlusDi", &[14.0]),
    ("PlusDm", &[14.0]),
    ("ProfileShape", &[3.0, 7.0]),
    ("ProjectionOscillator", &[14.0]),
    ("Psar", &[0.02, 0.02, 0.2]),
    ("Pvi", &[]),
    ("Qstick", &[14.0]),
    ("RectangleRange", &[]),
    ("RickshawMan", &[]),
    ("RisingThreeMethods", &[]),
    ("RogersSatchellVolatility", &[20.0, 252.0]),
    ("RollingVwap", &[14.0]),
    ("Rvi", &[14.0]),
    ("SarExt", &[2.0, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5]),
    ("SeasonalZScore", &[14.0]),
    ("SeparatingLines", &[]),
    ("SessionVwap", &[14.0]),
    ("Shark", &[]),
    ("ShootingStar", &[]),
    ("ShortLine", &[]),
    ("SinglePrints", &[3.0, 7.0]),
    ("Smi", &[3.0, 7.0, 14.0]),
    ("SpinningTop", &[]),
    ("StalledPattern", &[]),
    ("StickSandwich", &[]),
    ("StochasticCci", &[14.0]),
    ("Takuri", &[]),
    ("TasukiGap", &[]),
    ("TdCamouflage", &[]),
    ("TdClop", &[]),
    ("TdClopwin", &[]),
    ("TdCombo", &[3.0, 7.0, 14.0, 28.0]),
    ("TdCountdown", &[3.0, 7.0, 14.0, 28.0]),
    ("TdDWave", &[2.0]),
    ("TdDeMarker", &[14.0]),
    ("TdDifferential", &[]),
    ("TdOpen", &[]),
    ("TdPressure", &[14.0]),
    ("TdPropulsion", &[]),
    ("TdRei", &[14.0]),
    ("TdSetup", &[3.0, 7.0]),
    ("TdTrap", &[]),
    ("ThreeDrives", &[]),
    ("ThreeInside", &[]),
    ("ThreeLineBreak", &[14.0]),
    ("ThreeLineStrike", &[]),
    ("ThreeOutside", &[]),
    ("ThreeSoldiersOrCrows", &[]),
    ("ThreeStarsInSouth", &[]),
    ("Thrusting", &[]),
    ("TimeBasedStop", &[14.0]),
    ("TowerTopBottom", &[]),
    ("TradeVolumeIndex", &[2.0]),
    ("Triangle", &[]),
    ("TripleTopBottom", &[]),
    ("Tristar", &[]),
    ("TrueRange", &[]),
    ("Tsv", &[14.0]),
    ("TtmTrend", &[14.0]),
    ("TurnOfMonth", &[3.0, 3.0, 0.0]),
    ("Tweezer", &[]),
    ("TwiggsMoneyFlow", &[14.0]),
    ("TwoCrows", &[]),
    ("TypicalPrice", &[]),
    ("UltimateOscillator", &[3.0, 7.0, 14.0]),
    ("UniqueThreeRiver", &[]),
    ("UpsideGapThreeMethods", &[]),
    ("UpsideGapTwoCrows", &[]),
    ("VolatilityRatio", &[14.0]),
    ("VoltyStop", &[14.0, 2.0]),
    ("VolumeOscillator", &[3.0, 7.0]),
    ("VolumePriceTrend", &[]),
    ("VolumeRsi", &[14.0]),
    ("Vwap", &[]),
    ("Vwma", &[14.0]),
    ("Vzo", &[14.0]),
    ("Wad", &[]),
    ("Wedge", &[]),
    ("WeightedClose", &[]),
    ("WickRatio", &[]),
    ("WilliamsR", &[14.0]),
    ("YangZhangVolatility", &[20.0, 252.0]),
    ("YoyoExit", &[14.0, 2.0]),
    ("AccelerationBands", &[14.0, 2.0]),
    ("Adx", &[14.0]),
    ("Alligator", &[3.0, 7.0, 14.0]),
    ("AndrewsPitchfork", &[14.0]),
    ("Aroon", &[14.0]),
    ("AtrBands", &[14.0, 2.0]),
    ("AtrRatchet", &[14.0, 2.0, 0.5]),
    ("AutoFib", &[]),
    ("BollingerBands", &[20.0, 2.0]),
    ("BomarBands", &[4.0, 0.85]),
    ("Camarilla", &[]),
    ("CandleVolume", &[14.0]),
    ("CentralPivotRange", &[]),
    ("ChandeKrollStop", &[3.0, 2.0, 7.0]),
    ("ChandelierExit", &[14.0, 2.0]),
    ("ClassicPivots", &[]),
    ("CompositeProfile", &[20.0, 24.0, 0.7]),
    ("DemarkPivots", &[]),
    ("Donchian", &[14.0]),
    ("DonchianStop", &[14.0]),
    ("DoubleBollinger", &[20.0, 1.0, 2.0]),
    ("ElderRay", &[14.0]),
    ("ElderSafeZone", &[10.0, 2.0]),
    ("Equivolume", &[14.0]),
    ("FibArcs", &[]),
    ("FibChannel", &[]),
    ("FibConfluence", &[]),
    ("FibExtension", &[]),
    ("FibFan", &[]),
    ("FibProjection", &[]),
    ("FibRetracement", &[]),
    ("FibTimeZones", &[]),
    ("FibonacciPivots", &[]),
    ("FractalChaosBands", &[14.0]),
    ("GatorOscillator", &[3.0, 7.0, 14.0]),
    ("GoldenPocket", &[]),
    ("HeikinAshi", &[]),
    ("HighLowVolumeNodes", &[3.0, 7.0]),
    ("HtPhasor", &[]),
    ("HurstChannel", &[14.0, 2.0]),
    ("InitialBalance", &[14.0]),
    ("KaseDevStop", &[14.0, 2.0]),
    ("KasePermissionStochastic", &[3.0, 7.0]),
    ("Keltner", &[3.0, 7.0, 2.0]),
    ("Kst", &[3.0, 7.0, 14.0, 28.0, 35.0, 42.0, 56.0, 63.0, 70.0]),
    ("LinRegChannel", &[14.0, 2.0]),
    ("MaEnvelope", &[14.0, 2.0]),
    ("MacdFix", &[9.0]),
    ("MacdIndicator", &[12.0, 26.0, 9.0]),
    ("Mama", &[0.5, 0.05]),
    ("MedianChannel", &[14.0, 2.0]),
    ("ModifiedMaStop", &[14.0]),
    ("MurreyMathLines", &[14.0]),
    ("Nrtr", &[2.0]),
    ("OpeningRange", &[14.0]),
    ("OvernightIntradayReturn", &[14.0]),
    ("ProjectionBands", &[14.0]),
    ("Qqe", &[3.0, 7.0, 2.0]),
    ("QuartileBands", &[14.0]),
    ("Rwi", &[14.0]),
    ("SessionHighLow", &[14.0]),
    ("SessionRange", &[14.0]),
    ("SmoothedHeikinAshi", &[14.0]),
    ("StandardErrorBands", &[14.0, 2.0]),
    ("StarcBands", &[3.0, 7.0, 2.0]),
    ("Stochastic", &[3.0, 7.0]),
    ("SuperTrend", &[14.0, 2.0]),
    ("TdLines", &[3.0, 7.0]),
    ("TdMovingAverage", &[3.0, 7.0]),
    ("TdRangeProjection", &[]),
    ("TdRiskLevel", &[3.0, 7.0]),
    ("TdSequential", &[3.0, 7.0, 14.0, 28.0]),
    ("TpoProfile", &[14.0, 14.0]),
    ("TtmSqueeze", &[14.0, 2.0, 0.5]),
    ("ValueArea", &[20.0, 50.0, 0.7]),
    ("VolatilityCone", &[3.0, 7.0]),
    ("VolumeProfile", &[14.0, 14.0]),
    ("VolumeWeightedMacd", &[3.0, 7.0, 14.0]),
    ("VolumeWeightedSr", &[14.0]),
    ("Vortex", &[14.0]),
    ("VwapStdDevBands", &[2.0]),
    ("WaveTrend", &[3.0, 7.0, 14.0]),
    ("WoodiePivots", &[]),
    ("ZeroLagMacd", &[3.0, 7.0, 14.0]),
    ("ZigZag", &[0.02]),
    ("Alpha", &[14.0, 2.0]),
    ("Beta", &[14.0]),
    ("BetaNeutralSpread", &[14.0]),
    ("DistanceSsd", &[14.0]),
    ("GrangerCausality", &[60.0, 1.0]),
    ("HasbrouckInformationShare", &[14.0]),
    ("InformationRatio", &[14.0]),
    ("KendallTau", &[14.0]),
    ("OuHalfLife", &[14.0]),
    ("PairSpreadZScore", &[20.0, 20.0]),
    ("PairwiseBeta", &[14.0]),
    ("PearsonCorrelation", &[14.0]),
    ("RollingCorrelation", &[14.0]),
    ("RollingCovariance", &[14.0]),
    ("SpearmanCorrelation", &[14.0]),
    ("SpreadAr1Coefficient", &[14.0]),
    ("SpreadHurst", &[14.0]),
    ("TreynorRatio", &[14.0, 2.0]),
    ("VarianceRatio", &[60.0, 2.0]),
    ("Cointegration", &[40.0, 1.0]),
    ("KalmanHedgeRatio", &[0.01, 0.001]),
    ("LeadLagCrossCorrelation", &[20.0, 10.0]),
    ("RelativeStrengthAB", &[14.0, 14.0]),
    ("SpreadBollingerBands", &[14.0, 2.0]),
    ("CalendarSpread", &[]),
    ("EstimatedLeverageRatio", &[]),
    ("FundingBasis", &[]),
    ("FundingImpliedApr", &[2.0]),
    ("FundingRate", &[]),
    ("FundingRateMean", &[14.0]),
    ("FundingRateZScore", &[14.0]),
    ("LongShortRatio", &[]),
    ("OIPriceDivergence", &[14.0]),
    ("OIWeighted", &[]),
    ("OiToVolumeRatio", &[]),
    ("OpenInterestDelta", &[]),
    ("OpenInterestMomentum", &[14.0]),
    ("PerpetualPremiumIndex", &[]),
    ("TakerBuySellRatio", &[]),
    ("TermStructureBasis", &[]),
    ("LiquidationFeatures", &[]),
    ("DepthSlope", &[]),
    ("Microprice", &[]),
    ("OrderBookImbalanceFull", &[]),
    ("OrderBookImbalanceTop1", &[]),
    ("OrderBookImbalanceTopN", &[14.0]),
    ("OrderFlowImbalance", &[14.0]),
    ("QuotedSpread", &[]),
    ("AmihudIlliquidity", &[14.0]),
    ("CumulativeVolumeDelta", &[]),
    ("Pin", &[14.0]),
    ("RollMeasure", &[14.0]),
    ("SignedVolume", &[]),
    ("TradeImbalance", &[14.0]),
    ("TradeSignAutocorrelation", &[14.0]),
    ("Vpin", &[2.0, 14.0]),
    ("EffectiveSpread", &[]),
    ("KylesLambda", &[14.0]),
    ("RealizedSpread", &[14.0]),
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

    fn input(c: &Candle) -> BarInput<'_> {
        BarInput {
            candle: c,
            reference: None,
            deriv: None,
            orderbook: None,
            trades: &[],
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
        assert!(
            ALL_SPECS.len() >= 400,
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
        assert!(build("MacdIndicator", &[12.0, 26.0]).is_err()); // missing signal
        assert!(build("BollingerBands", &[20.0]).is_err()); // missing multiplier
    }

    #[test]
    fn aliases_resolve() {
        assert!(build("Macd", &[12.0, 26.0, 9.0]).is_ok());
        assert!(build("Bollinger", &[20.0, 2.0]).is_ok());
    }

    #[test]
    fn macd_exposes_fields() {
        let mut macd = build("MacdIndicator", &[2.0, 3.0, 2.0]).unwrap();
        let mut last_fields = Vec::new();
        for px in [10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0] {
            if macd.update(&input(&candle(px, px, px))).is_some() {
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
        sma.update(&input(&candle(10.0, 10.0, 10.0)));
        sma.update(&input(&candle(20.0, 20.0, 20.0)));
        assert!(sma.fields().is_empty());
    }
}
