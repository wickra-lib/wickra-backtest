//! A single JSON request bundling candles, the strategy spec and optional feeds
//! — the uniform entry point the language bindings call, so every binding can
//! run any feed combination by passing one JSON document.

use serde::Deserialize;

use crate::data::{Candle, CrossSection, DerivativesTick, OrderBook, TradePrint};
use crate::engine::{Feeds, StreamingBacktest, DEFAULT_CAPITAL};
use crate::error::{BacktestError, Result};
use crate::report::BacktestReport;
use crate::spec::StrategySpec;

fn default_capital() -> f64 {
    DEFAULT_CAPITAL
}

/// A complete backtest request: the strategy, the candle stream, the starting
/// capital and any optional per-bar feeds. Each present feed must be the same
/// length as `candles`.
#[derive(Debug, Clone, Deserialize)]
pub struct RunRequest {
    /// The strategy spec.
    pub spec: StrategySpec,
    /// The OHLCV candle stream.
    pub candles: Vec<Candle>,
    /// Starting capital (defaults to [`DEFAULT_CAPITAL`]).
    #[serde(default = "default_capital")]
    pub capital: f64,
    /// Reference candle series for pairwise indicators (its per-bar close).
    #[serde(default)]
    pub reference: Option<Vec<Candle>>,
    /// Per-bar derivatives ticks for derivatives indicators / funding.
    #[serde(default)]
    pub derivs: Option<Vec<DerivativesTick>>,
    /// Per-bar order-book snapshots for order-book / spread indicators.
    #[serde(default)]
    pub books: Option<Vec<OrderBook>>,
    /// Per-bar trade lists for trade-flow / trade-quote indicators.
    #[serde(default)]
    pub trades: Option<Vec<Vec<TradePrint>>>,
    /// Per-bar market cross-sections for breadth indicators.
    #[serde(default)]
    pub sections: Option<Vec<CrossSection>>,
}

impl RunRequest {
    /// Run the backtest, threading any present feeds bar by bar.
    pub fn run(&self) -> Result<BacktestReport> {
        self.spec.validate()?;
        let n = self.candles.len();
        if n == 0 {
            return Err(BacktestError::InvalidData("no candles".into()));
        }
        let check = |name: &str, len: Option<usize>| -> Result<()> {
            match len {
                Some(l) if l != n => Err(BacktestError::InvalidData(format!(
                    "{name} feed length {l} does not match {n} candles"
                ))),
                _ => Ok(()),
            }
        };
        check("reference", self.reference.as_ref().map(Vec::len))?;
        check("derivs", self.derivs.as_ref().map(Vec::len))?;
        check("books", self.books.as_ref().map(Vec::len))?;
        check("trades", self.trades.as_ref().map(Vec::len))?;
        check("sections", self.sections.as_ref().map(Vec::len))?;

        let mut bt = StreamingBacktest::new(&self.spec, self.capital)?;
        for (i, candle) in self.candles.iter().enumerate() {
            let feeds = Feeds {
                reference: self.reference.as_ref().map(|r| r[i].close),
                deriv: self.derivs.as_ref().map(|d| &d[i]),
                orderbook: self.books.as_ref().map(|b| &b[i]),
                trades: self.trades.as_ref().map(|t| t[i].as_slice()),
                cross_section: self.sections.as_ref().map(|s| &s[i]),
            };
            bt.step_with_feeds(candle, &feeds)?;
        }
        Ok(bt.finish())
    }
}

/// Run a backtest from a single JSON [`RunRequest`], returning the report JSON.
/// This is the uniform entry point every language binding wraps.
pub fn run_json(request_json: &str) -> Result<String> {
    let req: RunRequest = serde_json::from_str(request_json)
        .map_err(|e| BacktestError::InvalidSpec(e.to_string()))?;
    let report = req.run()?;
    serde_json::to_string(&report).map_err(|e| BacktestError::InvalidData(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_json_matches_plain_run() {
        let request = r#"{
            "capital": 1000.0,
            "spec": {"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},100]},
                "exit":{"lt":[{"price":"close"},100]},
                "sizing":{"type":"fixed_qty","qty":1}},
            "candles": [
                {"time":0,"open":100,"high":101,"low":100,"close":101},
                {"time":1,"open":102,"high":103,"low":102,"close":103},
                {"time":2,"open":104,"high":104,"low":99,"close":99},
                {"time":3,"open":98,"high":98,"low":97,"close":97}
            ]
        }"#;
        let json = run_json(request).unwrap();
        assert!(json.contains("\"num_trades\":1"));
        assert!(json.contains("\"entry_price\":102.0"));
        assert!(json.contains("\"exit_price\":98.0"));
    }

    #[test]
    fn run_json_threads_a_derivatives_feed() {
        let request = r#"{
            "spec": {"symbol":"x","timeframe":"1h",
                "indicators":{"f":{"type":"FundingRate","params":[]}},
                "entry":{"gt":["f",0.0]},"exit":{"lt":["f",-1.0]},
                "sizing":{"type":"fixed_qty","qty":1}},
            "candles": [
                {"time":0,"open":100,"high":100,"low":100,"close":100},
                {"time":1,"open":100,"high":100,"low":100,"close":100},
                {"time":2,"open":100,"high":100,"low":100,"close":100}
            ],
            "derivs": [
                {"funding_rate":0.01,"mark_price":100,"index_price":100,"futures_price":100,"open_interest":1000,"long_size":600,"short_size":400,"taker_buy_volume":50,"taker_sell_volume":40,"long_liquidation":0,"short_liquidation":0},
                {"funding_rate":0.01,"mark_price":100,"index_price":100,"futures_price":100,"open_interest":1000,"long_size":600,"short_size":400,"taker_buy_volume":50,"taker_sell_volume":40,"long_liquidation":0,"short_liquidation":0},
                {"funding_rate":0.01,"mark_price":100,"index_price":100,"futures_price":100,"open_interest":1000,"long_size":600,"short_size":400,"taker_buy_volume":50,"taker_sell_volume":40,"long_liquidation":0,"short_liquidation":0}
            ]
        }"#;
        let report = run_json(request).unwrap();
        assert!(report.contains("\"num_trades\":1"));
    }

    #[test]
    fn run_json_rejects_feed_length_mismatch() {
        let request = r#"{
            "spec": {"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},"exit":{"in_position":true},
                "sizing":{"type":"fixed_qty","qty":1}},
            "candles": [{"time":0,"open":1,"high":1,"low":1,"close":1}],
            "trades": []
        }"#;
        assert!(run_json(request).is_err());
    }
}
