//! The event-driven backtest loop.
//!
//! Look-ahead bias is structurally prevented: a signal is decided on a bar's
//! **close**, and the resulting order fills on the **next bar's open**. Equity is
//! marked to market at every close. This is the MVP engine — long-only,
//! market orders, fixed fee + fixed-bps slippage, close-based stop/target.

use std::collections::BTreeMap;

use crate::data::Candle;
use crate::error::{BacktestError, Result};
use crate::metrics;
use crate::portfolio::Portfolio;
use crate::registry::{self, EvalIndicator};
use crate::report::{BacktestReport, EquityPoint, REPORT_SCHEMA_VERSION};
use crate::rules::{eval_condition, BarRow, RuleState};
use crate::spec::{Sizing, Slippage, StrategySpec};

/// Default starting capital for the MVP runner.
pub const DEFAULT_CAPITAL: f64 = 10_000.0;

enum Action {
    Enter,
    Exit(&'static str),
}

/// Run a backtest of `spec` over `candles` with the default capital.
pub fn run(spec: &StrategySpec, candles: &[Candle]) -> Result<BacktestReport> {
    run_with_capital(spec, candles, DEFAULT_CAPITAL)
}

/// Run a backtest with explicit starting `capital`.
pub fn run_with_capital(
    spec: &StrategySpec,
    candles: &[Candle],
    capital: f64,
) -> Result<BacktestReport> {
    spec.validate()?;
    if candles.is_empty() {
        return Err(BacktestError::InvalidData("no candles".into()));
    }

    let mut indicators: BTreeMap<String, Box<dyn EvalIndicator>> = BTreeMap::new();
    let mut max_warmup = 0usize;
    for (name, ind) in &spec.indicators {
        let built = registry::build(&ind.kind, &ind.params)?;
        max_warmup = max_warmup.max(built.warmup());
        indicators.insert(name.clone(), built);
    }
    let warmup = spec.warmup.map_or(max_warmup, |w| w as usize);

    let taker = spec.costs.taker_bps / 10_000.0;
    let slip = match spec.costs.slippage {
        Slippage::FixedBps { bps } => bps / 10_000.0,
        // Spread / volume-impact slippage need feeds the MVP does not model yet.
        Slippage::Spread | Slippage::VolumeImpact { .. } => 0.0,
    };

    let mut pf = Portfolio::new(capital);
    let mut history: Vec<BarRow> = Vec::with_capacity(candles.len());
    let mut equity: Vec<EquityPoint> = Vec::with_capacity(candles.len());
    let mut pending: Option<Action> = None;
    let mut entry_bar: Option<usize> = None;

    for (t, candle) in candles.iter().enumerate() {
        // 1. Fill the pending order at this bar's open (look-ahead-free).
        match pending.take() {
            Some(Action::Enter) => {
                let fill = candle.open * (1.0 + slip);
                if let Some(qty) = size(spec.sizing, pf.cash, fill)? {
                    if qty > 0.0 {
                        let fee = qty * fill * taker;
                        pf.enter(qty, fill, candle.time, fee);
                        entry_bar = Some(t);
                    }
                }
            }
            Some(Action::Exit(reason)) if pf.in_position() => {
                let fill = candle.open * (1.0 - slip);
                let fee = pf.qty * fill * taker;
                pf.exit(fill, candle.time, fee, reason);
                entry_bar = None;
            }
            _ => {}
        }

        // 2. Update indicators and record the bar.
        let mut values = BTreeMap::new();
        for (name, ind) in &mut indicators {
            if let Some(v) = ind.update(candle) {
                values.insert(name.clone(), v);
            }
        }
        history.push(BarRow {
            candle: *candle,
            values,
        });
        let idx = history.len() - 1;

        // 3. Mark equity at the close.
        equity.push(EquityPoint {
            time: candle.time,
            equity: pf.equity(candle.close),
        });

        // 4. Decide the next action (fills at the next open). Skip warmup.
        if idx < warmup {
            continue;
        }
        let bars_since_entry = entry_bar.map(|e| (idx - e) as u32);
        let state = RuleState {
            in_position: pf.in_position(),
            bars_since_entry,
        };

        if pf.in_position() {
            if let Some(reason) = stop_hit(spec, pf.entry_price, candle.close) {
                pending = Some(Action::Exit(reason));
            } else if eval_condition(&spec.exit, &history, idx, state) {
                pending = Some(Action::Exit("signal"));
            }
        } else if eval_condition(&spec.entry, &history, idx, state) {
            pending = Some(Action::Enter);
        }
    }

    // Close any position still open at the final close.
    if pf.in_position() {
        let last = candles.last().expect("candles is non-empty");
        let fee = pf.qty * last.close * taker;
        pf.exit(last.close, last.time, fee, "end");
    }

    let series: Vec<f64> = equity.iter().map(|e| e.equity).collect();
    let metrics = metrics::compute(capital, &series, &pf.trades);
    Ok(BacktestReport {
        schema_version: REPORT_SCHEMA_VERSION,
        metrics,
        trades: pf.trades,
        equity,
        fees_paid: pf.fees_paid,
        initial_capital: capital,
    })
}

/// Quantity to buy given the sizing model, available cash and fill price.
fn size(sizing: Sizing, cash: f64, price: f64) -> Result<Option<f64>> {
    if price <= 0.0 {
        return Ok(None);
    }
    let qty = match sizing {
        Sizing::FixedFraction { fraction } => (cash * fraction) / price,
        Sizing::FixedCash { cash: notional } => notional.min(cash) / price,
        Sizing::FixedQty { qty } => qty,
        Sizing::VolTarget { .. } | Sizing::RiskPerTrade { .. } => {
            return Err(BacktestError::InvalidSpec(
                "vol_target / risk_per_trade sizing is not supported in the MVP".into(),
            ));
        }
    };
    Ok(Some(qty))
}

/// Close-based stop-loss / take-profit (intrabar fills come in Phase 3).
fn stop_hit(spec: &StrategySpec, entry: f64, close: f64) -> Option<&'static str> {
    if entry <= 0.0 {
        return None;
    }
    let change = (close - entry) / entry * 100.0;
    if let Some(sl) = spec.risk.stop_loss_pct {
        if change <= -sl {
            return Some("stop_loss");
        }
    }
    if let Some(tp) = spec.risk.take_profit_pct {
        if change >= tp {
            return Some("take_profit");
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::StrategySpec;

    fn bar(time: i64, open: f64, high: f64, low: f64, close: f64) -> Candle {
        Candle {
            time,
            open,
            high,
            low,
            close,
            volume: 0.0,
        }
    }

    /// A price-threshold strategy with no costs, hand-computed end to end.
    #[test]
    fn hand_computed_round_trip() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},100]},
                "exit":{"lt":[{"price":"close"},100]},
                "sizing":{"type":"fixed_qty","qty":1}}"#,
        )
        .unwrap();
        let candles = [
            bar(0, 100.0, 101.0, 100.0, 101.0), // close 101 > 100 -> signal enter
            bar(1, 102.0, 103.0, 102.0, 103.0), // fill enter @ open 102
            bar(2, 104.0, 104.0, 99.0, 99.0),   // close 99 < 100 -> signal exit
            bar(3, 98.0, 98.0, 97.0, 97.0),     // fill exit @ open 98
        ];
        let r = run_with_capital(&spec, &candles, 1000.0).unwrap();
        assert_eq!(r.trades.len(), 1);
        let t = &r.trades[0];
        assert!((t.entry_price - 102.0).abs() < 1e-9);
        assert!((t.exit_price - 98.0).abs() < 1e-9);
        assert!((t.pnl - (-4.0)).abs() < 1e-9);
        assert_eq!(t.reason, "signal");
        // final equity: 1000 - 102 + 98 = 996
        assert!((r.equity.last().unwrap().equity - 996.0).abs() < 1e-9);
        assert!((r.metrics.pnl - (-4.0)).abs() < 1e-9);
        assert_eq!(r.metrics.num_trades, 1);
    }

    #[test]
    fn no_signals_no_trades() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},1000000]},
                "exit":{"in_position":true},
                "sizing":{"type":"fixed_qty","qty":1}}"#,
        )
        .unwrap();
        let candles = [
            bar(0, 10.0, 10.0, 10.0, 10.0),
            bar(1, 11.0, 11.0, 11.0, 11.0),
        ];
        let r = run(&spec, &candles).unwrap();
        assert!(r.trades.is_empty());
        assert!((r.metrics.pnl).abs() < 1e-9);
    }

    #[test]
    fn open_position_closed_at_end() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},
                "exit":{"lt":[{"price":"close"},0]},
                "sizing":{"type":"fixed_qty","qty":1}}"#,
        )
        .unwrap();
        let candles = [
            bar(0, 10.0, 10.0, 10.0, 10.0),
            bar(1, 11.0, 11.0, 11.0, 11.0),
        ];
        let r = run_with_capital(&spec, &candles, 1000.0).unwrap();
        assert_eq!(r.trades.len(), 1);
        assert_eq!(r.trades[0].reason, "end"); // never exits by signal -> closed at last close
    }

    #[test]
    fn sma_crossover_runs() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h",
                "indicators":{"fast":{"type":"Sma","params":[2]},"slow":{"type":"Sma","params":[3]}},
                "entry":{"cross_above":["fast","slow"]},
                "exit":{"cross_below":["fast","slow"]},
                "sizing":{"type":"fixed_fraction","fraction":0.5}}"#,
        )
        .unwrap();
        let candles: Vec<Candle> = (0..20)
            .map(|i| {
                bar(
                    i,
                    100.0 + i as f64,
                    100.0 + i as f64,
                    100.0,
                    100.0 + i as f64,
                )
            })
            .collect();
        let r = run(&spec, &candles).unwrap();
        assert_eq!(r.equity.len(), 20);
        assert_eq!(r.schema_version, REPORT_SCHEMA_VERSION);
    }

    #[test]
    fn unsupported_sizing_errors() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},
                "exit":{"in_position":true},
                "sizing":{"type":"vol_target","target_vol":0.2,"lookback":20}}"#,
        )
        .unwrap();
        let candles = [
            bar(0, 10.0, 10.0, 10.0, 10.0),
            bar(1, 11.0, 11.0, 11.0, 11.0),
        ];
        assert!(run(&spec, &candles).is_err());
    }
}
