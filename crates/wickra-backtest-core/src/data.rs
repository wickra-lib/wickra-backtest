//! Market-data input types fed to the engine.
//!
//! These are serde-friendly, owned value types (loadable from JSONL/CSV/Parquet)
//! that convert into the `wickra-core` input types when fed to indicators.
//! Besides OHLCV [`Candle`]s, the microstructure feed types — [`TradePrint`],
//! [`OrderBook`] and [`DerivativesTick`] — back the trade / order-book /
//! derivatives indicators.

use serde::{Deserialize, Serialize};
use wickra_core::{
    Candle as CoreCandle, DerivativesTick as CoreDerivativesTick, Level as CoreLevel,
    OrderBook as CoreOrderBook, Side as CoreSide, Trade as CoreTrade,
};

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

/// Aggressor side of a [`TradePrint`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeSide {
    /// A buyer-initiated (aggressive buy) trade.
    Buy,
    /// A seller-initiated (aggressive sell) trade.
    Sell,
}

impl TradeSide {
    fn to_core(self) -> CoreSide {
        match self {
            TradeSide::Buy => CoreSide::Buy,
            TradeSide::Sell => CoreSide::Sell,
        }
    }
}

/// A single trade print, fed to trade-flow indicators.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TradePrint {
    /// Execution price (strictly positive).
    pub price: f64,
    /// Executed size / quantity (non-negative).
    pub size: f64,
    /// Aggressor side.
    pub side: TradeSide,
    /// Trade timestamp (engine-defined epoch unit).
    #[serde(default)]
    pub timestamp: i64,
}

impl TradePrint {
    /// Convert into a `wickra-core` trade, validating price/size.
    pub fn to_core(self) -> Result<CoreTrade> {
        CoreTrade::new(self.price, self.size, self.side.to_core(), self.timestamp)
            .map_err(|e| BacktestError::InvalidData(e.to_string()))
    }
}

/// One order-book price level.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Level {
    /// Price of the level (strictly positive).
    pub price: f64,
    /// Resting size / quantity at this price (non-negative).
    pub size: f64,
}

impl Level {
    fn to_core(self) -> Result<CoreLevel> {
        CoreLevel::new(self.price, self.size).map_err(|e| BacktestError::InvalidData(e.to_string()))
    }
}

/// An order-book snapshot (best level first on each side), fed to order-book
/// indicators.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderBook {
    /// Bid levels, best (highest price) first.
    pub bids: Vec<Level>,
    /// Ask levels, best (lowest price) first.
    pub asks: Vec<Level>,
}

impl OrderBook {
    /// Convert into a `wickra-core` order book, validating the level and
    /// ordering invariants (non-empty, sorted, uncrossed).
    pub fn to_core(&self) -> Result<CoreOrderBook> {
        let bids = self
            .bids
            .iter()
            .map(|l| l.to_core())
            .collect::<Result<Vec<_>>>()?;
        let asks = self
            .asks
            .iter()
            .map(|l| l.to_core())
            .collect::<Result<Vec<_>>>()?;
        CoreOrderBook::new(bids, asks).map_err(|e| BacktestError::InvalidData(e.to_string()))
    }
}

/// A derivatives (perpetual / futures) tick, fed to derivatives indicators.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DerivativesTick {
    /// Funding rate for the interval (finite; may be negative).
    pub funding_rate: f64,
    /// Perpetual mark price (strictly positive).
    pub mark_price: f64,
    /// Spot / index price the perpetual tracks (strictly positive).
    pub index_price: f64,
    /// Dated futures mark price (strictly positive).
    pub futures_price: f64,
    /// Open interest (non-negative).
    pub open_interest: f64,
    /// Aggregate long size (non-negative).
    pub long_size: f64,
    /// Aggregate short size (non-negative).
    pub short_size: f64,
    /// Taker buy volume (non-negative).
    pub taker_buy_volume: f64,
    /// Taker sell volume (non-negative).
    pub taker_sell_volume: f64,
    /// Long-liquidation volume (non-negative).
    pub long_liquidation: f64,
    /// Short-liquidation volume (non-negative).
    pub short_liquidation: f64,
    /// Tick timestamp (engine-defined epoch unit).
    #[serde(default)]
    pub timestamp: i64,
}

impl DerivativesTick {
    /// Convert into a `wickra-core` derivatives tick, validating the fields.
    pub fn to_core(self) -> Result<CoreDerivativesTick> {
        CoreDerivativesTick::new(
            self.funding_rate,
            self.mark_price,
            self.index_price,
            self.futures_price,
            self.open_interest,
            self.long_size,
            self.short_size,
            self.taker_buy_volume,
            self.taker_sell_volume,
            self.long_liquidation,
            self.short_liquidation,
            self.timestamp,
        )
        .map_err(|e| BacktestError::InvalidData(e.to_string()))
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

    #[test]
    fn trade_converts_and_validates() {
        let t = TradePrint {
            price: 100.0,
            size: 1.5,
            side: TradeSide::Buy,
            timestamp: 7,
        };
        assert!(t.to_core().is_ok());
        let bad = TradePrint { price: -1.0, ..t };
        assert!(bad.to_core().is_err());
    }

    #[test]
    fn trade_deserializes_side() {
        let t: TradePrint =
            serde_json::from_str(r#"{"price":100,"size":1,"side":"sell"}"#).unwrap();
        assert_eq!(t.side, TradeSide::Sell);
        assert_eq!(t.timestamp, 0); // defaulted
    }

    #[test]
    fn order_book_converts_and_rejects_crossed() {
        let ob = OrderBook {
            bids: vec![Level {
                price: 100.0,
                size: 2.0,
            }],
            asks: vec![Level {
                price: 101.0,
                size: 3.0,
            }],
        };
        assert!(ob.to_core().is_ok());
        let crossed = OrderBook {
            bids: vec![Level {
                price: 102.0,
                size: 1.0,
            }],
            asks: vec![Level {
                price: 101.0,
                size: 1.0,
            }],
        };
        assert!(crossed.to_core().is_err());
    }

    #[test]
    fn derivatives_tick_converts() {
        let d = DerivativesTick {
            funding_rate: 0.0001,
            mark_price: 100.0,
            index_price: 99.9,
            futures_price: 100.5,
            open_interest: 1000.0,
            long_size: 600.0,
            short_size: 400.0,
            taker_buy_volume: 50.0,
            taker_sell_volume: 40.0,
            long_liquidation: 1.0,
            short_liquidation: 2.0,
            timestamp: 1,
        };
        assert!(d.to_core().is_ok());
    }
}
