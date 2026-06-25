//! The event-driven backtest loop.
//!
//! Look-ahead bias is structurally prevented: signal-driven orders are decided
//! on a bar's **close** and fill on the **next bar's open**. Stop-loss and
//! take-profit are price levels checked **intrabar** against each bar's OHLC and
//! fill at the level (conservative: the stop is assumed hit before the target
//! when a bar's range brackets both). Equity is marked to market at every close.
//!
//! Supports long and short positions, market orders, a taker fee and fixed-bps
//! slippage. Leverage, perp funding and liquidation come in a later phase.

use std::collections::BTreeMap;

use crate::data::Candle;
use crate::error::{BacktestError, Result};
use crate::metrics;
use crate::portfolio::Portfolio;
use crate::registry::{self, EvalIndicator};
use crate::report::{BacktestReport, EquityPoint, REPORT_SCHEMA_VERSION};
use crate::rules::{eval_condition, BarRow, RuleState};
use crate::spec::{Risk, Sizing, Slippage, StrategySpec};

/// Default starting capital for the runner.
pub const DEFAULT_CAPITAL: f64 = 10_000.0;

#[derive(Clone, Copy)]
enum Side {
    Long,
    Short,
}

enum Action {
    Enter(Side),
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
        // Spread / volume-impact slippage need feeds the engine does not model yet.
        Slippage::Spread | Slippage::VolumeImpact { .. } => 0.0,
    };

    let mut pf = Portfolio::new(capital);
    let mut history: Vec<BarRow> = Vec::with_capacity(candles.len());
    let mut equity: Vec<EquityPoint> = Vec::with_capacity(candles.len());
    let mut pending: Option<Action> = None;
    let mut entry_bar: Option<usize> = None;

    for (t, candle) in candles.iter().enumerate() {
        // 1. Fill the pending signal order at this bar's open (look-ahead-free).
        match pending.take() {
            Some(Action::Enter(side)) => {
                let dir = match side {
                    Side::Long => 1.0,
                    Side::Short => -1.0,
                };
                let fill = candle.open * (1.0 + dir * slip);
                if let Some(base) = size(spec.sizing, pf.cash, fill)? {
                    if base > 0.0 {
                        let fee = base * fill * taker;
                        pf.enter(dir * base, fill, candle.time, fee);
                        entry_bar = Some(t);
                    }
                }
            }
            Some(Action::Exit(reason)) if pf.in_position() => {
                // Long exit sells (fills lower), short exit buys (fills higher).
                let dir = if pf.is_long() { -1.0 } else { 1.0 };
                let fill = candle.open * (1.0 + dir * slip);
                let fee = pf.qty.abs() * fill * taker;
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

        // 3. Intrabar stop-loss / take-profit against this bar's OHLC.
        if pf.in_position() {
            if let Some((price, reason)) =
                intrabar_exit(candle, &spec.risk, pf.entry_price, pf.is_long())
            {
                let fee = pf.qty.abs() * price * taker;
                pf.exit(price, candle.time, fee, reason);
                entry_bar = None;
            }
        }

        // 4. Mark equity at the close.
        equity.push(EquityPoint {
            time: candle.time,
            equity: pf.equity(candle.close),
        });

        // 5. Decide the next signal action (fills at the next open). Skip warmup.
        if idx < warmup {
            continue;
        }
        let bars_since_entry = entry_bar.map(|e| (idx - e) as u32);
        let state = RuleState {
            in_position: pf.in_position(),
            bars_since_entry,
        };

        if pf.in_position() {
            let cond = if pf.is_long() {
                &spec.exit
            } else {
                spec.short_exit.as_ref().unwrap_or(&spec.exit)
            };
            if eval_condition(cond, &history, idx, state) {
                pending = Some(Action::Exit("signal"));
            }
        } else if eval_condition(&spec.entry, &history, idx, state) {
            pending = Some(Action::Enter(Side::Long));
        } else if let Some(short_entry) = &spec.short_entry {
            if eval_condition(short_entry, &history, idx, state) {
                pending = Some(Action::Enter(Side::Short));
            }
        }
    }

    // Close any position still open at the final close.
    if pf.in_position() {
        let last = candles.last().expect("candles is non-empty");
        let fee = pf.qty.abs() * last.close * taker;
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

/// Base (unsigned) quantity for the sizing model, given cash and fill price.
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
                "vol_target / risk_per_trade sizing is not supported yet".into(),
            ));
        }
    };
    Ok(Some(qty))
}

/// Intrabar stop-loss / take-profit fill against the bar's OHLC.
///
/// Conservative: when a bar's range brackets both the stop and the target, the
/// stop is assumed to fill first. Levels are side-aware (a short's stop is above
/// entry, its target below).
fn intrabar_exit(
    candle: &Candle,
    risk: &Risk,
    entry: f64,
    is_long: bool,
) -> Option<(f64, &'static str)> {
    if entry <= 0.0 {
        return None;
    }
    if is_long {
        if let Some(p) = risk.stop_loss_pct {
            let level = entry * (1.0 - p / 100.0);
            if candle.low <= level {
                return Some((level, "stop_loss"));
            }
        }
        if let Some(p) = risk.take_profit_pct {
            let level = entry * (1.0 + p / 100.0);
            if candle.high >= level {
                return Some((level, "take_profit"));
            }
        }
    } else {
        if let Some(p) = risk.stop_loss_pct {
            let level = entry * (1.0 + p / 100.0);
            if candle.high >= level {
                return Some((level, "stop_loss"));
            }
        }
        if let Some(p) = risk.take_profit_pct {
            let level = entry * (1.0 - p / 100.0);
            if candle.low <= level {
                return Some((level, "take_profit"));
            }
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

    /// A price-threshold long strategy with no costs, hand-computed end to end.
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
            bar(0, 100.0, 101.0, 100.0, 101.0),
            bar(1, 102.0, 103.0, 102.0, 103.0), // fill enter @ open 102
            bar(2, 104.0, 104.0, 99.0, 99.0),
            bar(3, 98.0, 98.0, 97.0, 97.0), // fill exit @ open 98
        ];
        let r = run_with_capital(&spec, &candles, 1000.0).unwrap();
        assert_eq!(r.trades.len(), 1);
        let t = &r.trades[0];
        assert!((t.entry_price - 102.0).abs() < 1e-9);
        assert!((t.exit_price - 98.0).abs() < 1e-9);
        assert!((t.pnl - (-4.0)).abs() < 1e-9);
        assert!((r.equity.last().unwrap().equity - 996.0).abs() < 1e-9);
    }

    /// Short entry profits when price falls; exit fills at next open.
    #[test]
    fn short_round_trip() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"lt":[{"price":"close"},0]},
                "exit":{"in_position":true},
                "short_entry":{"lt":[{"price":"close"},100]},
                "short_exit":{"gt":[{"price":"close"},100]},
                "sizing":{"type":"fixed_qty","qty":1}}"#,
        )
        .unwrap();
        let candles = [
            bar(0, 100.0, 100.0, 99.0, 99.0),   // close 99 < 100 -> short signal
            bar(1, 98.0, 98.0, 98.0, 98.0),     // fill short @ open 98
            bar(2, 101.0, 101.0, 101.0, 101.0), // close 101 > 100 -> cover signal
            bar(3, 102.0, 102.0, 102.0, 102.0), // fill cover @ open 102
        ];
        let r = run_with_capital(&spec, &candles, 1000.0).unwrap();
        assert_eq!(r.trades.len(), 1);
        let t = &r.trades[0];
        assert!((t.entry_price - 98.0).abs() < 1e-9);
        assert!((t.exit_price - 102.0).abs() < 1e-9);
        // short pnl = -1 * (102 - 98) = -4
        assert!((t.pnl - (-4.0)).abs() < 1e-9);
        assert_eq!(t.reason, "signal");
    }

    /// A long position whose stop is hit intrabar fills at the stop level.
    #[test]
    fn intrabar_stop_loss() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},
                "exit":{"lt":[{"price":"close"},0]},
                "sizing":{"type":"fixed_qty","qty":1},
                "risk":{"stop_loss_pct":5.0}}"#,
        )
        .unwrap();
        let candles = [
            bar(0, 100.0, 100.0, 100.0, 100.0), // enter signal
            bar(1, 100.0, 101.0, 100.0, 100.0), // fill enter @ 100; stop at 95
            bar(2, 99.0, 99.0, 90.0, 92.0),     // low 90 <= 95 -> stop fills @ 95
        ];
        let r = run_with_capital(&spec, &candles, 1000.0).unwrap();
        assert_eq!(r.trades.len(), 1);
        let t = &r.trades[0];
        assert!((t.exit_price - 95.0).abs() < 1e-9);
        assert_eq!(t.reason, "stop_loss");
        assert!((t.pnl - (-5.0)).abs() < 1e-9); // 1 * (95 - 100)
    }

    /// A long position whose target is hit intrabar fills at the target level.
    #[test]
    fn intrabar_take_profit() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},
                "exit":{"lt":[{"price":"close"},0]},
                "sizing":{"type":"fixed_qty","qty":1},
                "risk":{"take_profit_pct":10.0}}"#,
        )
        .unwrap();
        let candles = [
            bar(0, 100.0, 100.0, 100.0, 100.0), // enter signal
            bar(1, 100.0, 100.0, 100.0, 100.0), // fill enter @ 100; target 110
            bar(2, 105.0, 115.0, 105.0, 112.0), // high 115 >= 110 -> target fills @ 110
        ];
        let r = run_with_capital(&spec, &candles, 1000.0).unwrap();
        assert_eq!(r.trades.len(), 1);
        let t = &r.trades[0];
        assert!((t.exit_price - 110.0).abs() < 1e-9);
        assert_eq!(t.reason, "take_profit");
        assert!((t.pnl - 10.0).abs() < 1e-9);
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
        assert_eq!(r.trades[0].reason, "end");
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
