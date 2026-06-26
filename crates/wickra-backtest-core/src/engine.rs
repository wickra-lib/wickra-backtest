//! The event-driven backtest loop.
//!
//! Look-ahead bias is structurally prevented by default: signal-driven orders
//! are decided on a bar's **close** and fill on the **next bar's open**. An
//! opt-in `fill_timing: "close"` instead fills market orders on the signalling
//! bar's own close (close-to-close, deliberately optimistic). Stop-loss and
//! take-profit are price levels checked **intrabar** against each bar's OHLC and
//! fill at the level (conservative: the stop is assumed hit before the target
//! when a bar's range brackets both). Equity is marked to market at every close.
//!
//! Supports long and short positions; market, limit and stop entry orders (a
//! limit/stop rests at a percent offset from the signal close and fills when a
//! later bar reaches it, otherwise it keeps working); maker/taker fees (resting
//! limit fills pay maker, market/stop fills pay taker) and fixed-bps
//! slippage; and leverage / position sizing (fixed fraction / cash / quantity,
//! risk-per-trade and vol-target, capped by `max_leverage` and
//! `max_position_pct`; without `max_leverage` the cap is 1x equity — no leverage
//! by default). Execution latency (`latency_bars`) delays every fill, and
//! volume-participation partial fills (`partial_fills` + `max_participation`)
//! cap an entry to a fraction of the bar's volume. Perpetual funding
//! (`costs.funding`) is charged each bar to an open position from the
//! derivatives feed, and a leveraged position is liquidated intrabar at its
//! bankruptcy price when `risk.liquidation` is set.

use std::collections::BTreeMap;

use crate::data::{Candle, DerivativesTick, OrderBook, TradePrint};
use crate::error::{BacktestError, Result};
use crate::metrics;
use crate::portfolio::Portfolio;
use crate::registry::{self, BarInput, EvalIndicator};
use crate::report::{BacktestReport, EquityPoint, REPORT_SCHEMA_VERSION};
use crate::rules::{eval_condition, BarRow, RuleState};
use crate::spec::{Execution, FillTiming, OrderType, Risk, Sizing, Slippage, StrategySpec};

/// Default starting capital for the runner.
pub const DEFAULT_CAPITAL: f64 = 10_000.0;

#[derive(Clone, Copy)]
enum Side {
    Long,
    Short,
}

/// A resting limit or stop trigger.
#[derive(Clone, Copy)]
enum LevelKind {
    Limit,
    Stop,
}

/// What a working order does once it fills.
enum Action {
    /// An entry. `trigger` is `None` for a market order (fills at the next
    /// open) or a resting limit/stop level (fills when the bar reaches it).
    Enter {
        side: Side,
        trigger: Option<(f64, LevelKind)>,
    },
    /// A market exit, fills at the next open.
    Exit(&'static str),
}

/// A working order, decided on a bar's close and filled on a later bar. `delay`
/// counts down the simulated execution latency before the order is eligible.
struct Pending {
    action: Action,
    delay: u32,
}

/// Fill price for a resting level order against a bar, or `None` if not reached.
/// A buy fills at the open when it gaps through the level (open below a limit,
/// above a stop), otherwise at the level; a sell mirrors this.
fn level_fill(side: Side, trigger: f64, kind: LevelKind, c: &Candle) -> Option<f64> {
    let is_buy = matches!(side, Side::Long);
    match (is_buy, kind) {
        (true, LevelKind::Limit) => (c.low <= trigger).then(|| c.open.min(trigger)),
        (true, LevelKind::Stop) => (c.high >= trigger).then(|| c.open.max(trigger)),
        (false, LevelKind::Limit) => (c.high >= trigger).then(|| c.open.max(trigger)),
        (false, LevelKind::Stop) => (c.low <= trigger).then(|| c.open.min(trigger)),
    }
}

/// The resting trigger level for an entry, or `None` for a market order. The
/// level is the signal bar's close shifted by the configured limit/stop offset.
fn entry_trigger(exec: &Execution, signal_close: f64) -> Option<(f64, LevelKind)> {
    match exec.order_type {
        OrderType::Limit => Some((
            signal_close * (1.0 + exec.limit_offset_pct.unwrap_or(0.0) / 100.0),
            LevelKind::Limit,
        )),
        OrderType::Stop => Some((
            signal_close * (1.0 + exec.stop_offset_pct.unwrap_or(0.0) / 100.0),
            LevelKind::Stop,
        )),
        // Market order (StopLimit is rejected by validation before the run).
        _ => None,
    }
}

/// Realized per-bar return volatility (standard deviation of simple
/// close-to-close returns) over the last `lookback` bars, or `None` if there is
/// not enough history or the series is flat.
fn realized_vol(history: &[BarRow], lookback: usize) -> Option<f64> {
    if lookback < 2 || history.len() < lookback {
        return None;
    }
    let closes: Vec<f64> = history[history.len() - lookback..]
        .iter()
        .map(|row| row.candle.close)
        .collect();
    let rets: Vec<f64> = closes
        .windows(2)
        .filter(|w| w[0].abs() > f64::EPSILON)
        .map(|w| (w[1] - w[0]) / w[0])
        .collect();
    if rets.is_empty() {
        return None;
    }
    let mean = rets.iter().sum::<f64>() / rets.len() as f64;
    let var = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / rets.len() as f64;
    let sd = var.sqrt();
    (sd > 0.0).then_some(sd)
}

/// Context an entry/exit fill needs from the run loop.
struct FillCtx<'a> {
    spec: &'a StrategySpec,
    candle: &'a Candle,
    history: &'a [BarRow],
    maker: f64,
    taker: f64,
    slip: f64,
    bar: usize,
}

/// Open a position at `raw_price` (before slippage), honouring the sizing model,
/// leverage caps and volume-participation partial fills. `maker_fill` charges
/// the maker fee (resting limit fills) instead of the taker fee.
fn execute_entry(
    side: Side,
    raw_price: f64,
    maker_fill: bool,
    ctx: &FillCtx,
    pf: &mut Portfolio,
    entry_bar: &mut Option<usize>,
    extreme: &mut f64,
) -> Result<()> {
    let dir = match side {
        Side::Long => 1.0,
        Side::Short => -1.0,
    };
    let fill = raw_price * (1.0 + dir * ctx.slip);
    let rv = match ctx.spec.sizing {
        Sizing::VolTarget { lookback, .. } => realized_vol(ctx.history, lookback as usize),
        _ => None,
    };
    if let Some(base) = size(ctx.spec.sizing, &ctx.spec.risk, pf.cash, fill, rv)? {
        // Immediate-or-cancel partial fills: take at most a participation cap of
        // the bar's volume.
        let base = if ctx.spec.execution.partial_fills {
            let cap = ctx.spec.execution.max_participation.unwrap_or(0.0) * ctx.candle.volume;
            base.min(cap)
        } else {
            base
        };
        if base > 0.0 {
            let rate = if maker_fill { ctx.maker } else { ctx.taker };
            let fee = base * fill * rate;
            pf.enter(dir * base, fill, ctx.candle.time, fee);
            *entry_bar = Some(ctx.bar);
            *extreme = fill;
        }
    }
    Ok(())
}

/// Close the open position at `raw_price` (before slippage).
fn execute_exit(
    reason: &'static str,
    raw_price: f64,
    ctx: &FillCtx,
    pf: &mut Portfolio,
    entry_bar: &mut Option<usize>,
) {
    if !pf.in_position() {
        return;
    }
    // Long exit sells (fills lower), short exit buys (fills higher).
    let dir = if pf.is_long() { -1.0 } else { 1.0 };
    let fill = raw_price * (1.0 + dir * ctx.slip);
    let fee = pf.qty.abs() * fill * ctx.taker;
    pf.exit(fill, ctx.candle.time, fee, reason);
    *entry_bar = None;
}

/// The optional non-OHLCV feeds for one bar: a reference-series close (pairwise),
/// a derivatives tick (derivatives) and an order-book snapshot (order-book).
/// Absent feeds are `None`; indicators that need a missing feed yield nothing.
#[derive(Default)]
pub struct Feeds<'a> {
    /// Reference-series close for pairwise indicators.
    pub reference: Option<f64>,
    /// Derivatives tick for derivatives indicators.
    pub deriv: Option<&'a DerivativesTick>,
    /// Order-book snapshot for order-book indicators.
    pub orderbook: Option<&'a OrderBook>,
    /// Trades that printed within this bar, for trade-flow indicators.
    pub trades: Option<&'a [TradePrint]>,
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
    let mut bt = StreamingBacktest::new(spec, capital)?;
    for candle in candles {
        bt.step(candle)?;
    }
    Ok(bt.finish())
}

/// Run a backtest with a reference price series for pairwise indicators. The
/// reference candle at each index supplies the second input (its close) to
/// pairwise indicators such as correlation, beta or spread. `reference` must be
/// the same length as `candles`.
pub fn run_with_ref(
    spec: &StrategySpec,
    candles: &[Candle],
    reference: &[Candle],
    capital: f64,
) -> Result<BacktestReport> {
    spec.validate()?;
    if candles.is_empty() {
        return Err(BacktestError::InvalidData("no candles".into()));
    }
    if reference.len() != candles.len() {
        return Err(BacktestError::InvalidData(
            "reference series must have the same length as the candles".into(),
        ));
    }
    let mut bt = StreamingBacktest::new(spec, capital)?;
    for (candle, ref_candle) in candles.iter().zip(reference) {
        bt.step_with_ref(candle, Some(ref_candle.close))?;
    }
    Ok(bt.finish())
}

/// Run a backtest with a per-bar derivatives feed for derivatives indicators
/// (funding, open interest, long/short ratio, …). `derivs` must be the same
/// length as `candles`.
pub fn run_with_deriv(
    spec: &StrategySpec,
    candles: &[Candle],
    derivs: &[DerivativesTick],
    capital: f64,
) -> Result<BacktestReport> {
    spec.validate()?;
    if candles.is_empty() {
        return Err(BacktestError::InvalidData("no candles".into()));
    }
    if derivs.len() != candles.len() {
        return Err(BacktestError::InvalidData(
            "derivatives feed must have the same length as the candles".into(),
        ));
    }
    let mut bt = StreamingBacktest::new(spec, capital)?;
    for (candle, d) in candles.iter().zip(derivs) {
        bt.step_with_feeds(
            candle,
            &Feeds {
                deriv: Some(d),
                ..Default::default()
            },
        )?;
    }
    Ok(bt.finish())
}

/// Run a backtest with a per-bar order-book feed for order-book indicators
/// (imbalance, microprice, quoted spread, …). `books` must be the same length
/// as `candles`.
pub fn run_with_orderbook(
    spec: &StrategySpec,
    candles: &[Candle],
    books: &[OrderBook],
    capital: f64,
) -> Result<BacktestReport> {
    spec.validate()?;
    if candles.is_empty() {
        return Err(BacktestError::InvalidData("no candles".into()));
    }
    if books.len() != candles.len() {
        return Err(BacktestError::InvalidData(
            "order-book feed must have the same length as the candles".into(),
        ));
    }
    let mut bt = StreamingBacktest::new(spec, capital)?;
    for (candle, ob) in candles.iter().zip(books) {
        bt.step_with_feeds(
            candle,
            &Feeds {
                orderbook: Some(ob),
                ..Default::default()
            },
        )?;
    }
    Ok(bt.finish())
}

/// Run a backtest with a per-bar trade feed for trade-flow indicators (CVD,
/// trade imbalance, VPIN, signed volume, …). `trades[i]` is the list of trades
/// that printed within bar `i`; the outer length must match `candles`.
pub fn run_with_trades(
    spec: &StrategySpec,
    candles: &[Candle],
    trades: &[Vec<TradePrint>],
    capital: f64,
) -> Result<BacktestReport> {
    spec.validate()?;
    if candles.is_empty() {
        return Err(BacktestError::InvalidData("no candles".into()));
    }
    if trades.len() != candles.len() {
        return Err(BacktestError::InvalidData(
            "trade feed must have one trade list per candle".into(),
        ));
    }
    let mut bt = StreamingBacktest::new(spec, capital)?;
    for (candle, bar_trades) in candles.iter().zip(trades) {
        bt.step_with_feeds(
            candle,
            &Feeds {
                trades: Some(bar_trades.as_slice()),
                ..Default::default()
            },
        )?;
    }
    Ok(bt.finish())
}

/// A streaming backtest: feed bars one at a time with [`StreamingBacktest::step`],
/// then [`StreamingBacktest::finish`]. The historical runner is exactly this fed
/// from a slice, so **backtest and live share one code path** — point `step` at
/// a live feed and the same engine becomes the live bot.
pub struct StreamingBacktest<'a> {
    spec: &'a StrategySpec,
    capital: f64,
    maker: f64,
    taker: f64,
    slip: f64,
    warmup: usize,
    indicators: BTreeMap<String, Box<dyn EvalIndicator>>,
    pf: Portfolio,
    history: Vec<BarRow>,
    equity: Vec<EquityPoint>,
    pending: Option<Pending>,
    entry_bar: Option<usize>,
    // Most favourable price reached since entry (peak for a long, trough for a
    // short) — the reference for the trailing stop.
    extreme: f64,
    // (time, close) of the most recent bar, for the final mark-out.
    last: Option<(i64, f64)>,
}

impl<'a> StreamingBacktest<'a> {
    /// Build a streaming backtest from a validated spec and starting capital.
    pub fn new(spec: &'a StrategySpec, capital: f64) -> Result<Self> {
        spec.validate()?;
        let mut indicators: BTreeMap<String, Box<dyn EvalIndicator>> = BTreeMap::new();
        let mut max_warmup = 0usize;
        for (name, ind) in &spec.indicators {
            let built = registry::build(&ind.kind, &ind.params)?;
            max_warmup = max_warmup.max(built.warmup());
            indicators.insert(name.clone(), built);
        }
        let warmup = spec.warmup.map_or(max_warmup, |w| w as usize);
        let maker = spec.costs.maker_bps / 10_000.0;
        let taker = spec.costs.taker_bps / 10_000.0;
        let slip = match spec.costs.slippage {
            Slippage::FixedBps { bps } => bps / 10_000.0,
            // Spread / volume-impact slippage need feeds the engine does not model yet.
            Slippage::Spread | Slippage::VolumeImpact { .. } => 0.0,
        };
        Ok(Self {
            spec,
            capital,
            maker,
            taker,
            slip,
            warmup,
            indicators,
            pf: Portfolio::new(capital),
            history: Vec::new(),
            equity: Vec::new(),
            pending: None,
            entry_bar: None,
            extreme: 0.0,
            last: None,
        })
    }

    /// Process one bar: fill the working order, update indicators, check intrabar
    /// stops, mark equity and decide the next action. Look-ahead-free.
    pub fn step(&mut self, candle: &Candle) -> Result<()> {
        self.step_with_feeds(candle, &Feeds::default())
    }

    /// Like [`StreamingBacktest::step`], but also supplies the reference series'
    /// close for this bar, which pairwise indicators consume as their second
    /// input. Single-instrument indicators ignore it.
    pub fn step_with_ref(&mut self, candle: &Candle, reference: Option<f64>) -> Result<()> {
        self.step_with_feeds(
            candle,
            &Feeds {
                reference,
                ..Default::default()
            },
        )
    }

    /// Process one bar with its optional non-OHLCV [`Feeds`]. Pairwise indicators
    /// consume the reference; derivatives / order-book indicators consume the
    /// tick / snapshot; other indicators ignore them.
    pub fn step_with_feeds(&mut self, candle: &Candle, feeds: &Feeds) -> Result<()> {
        let reference = feeds.reference;
        let deriv = feeds.deriv.and_then(|d| d.to_core().ok());
        let orderbook = feeds.orderbook.and_then(|ob| ob.to_core().ok());
        let trades: Vec<_> = feeds
            .trades
            .unwrap_or(&[])
            .iter()
            .filter_map(|tp| tp.to_core().ok())
            .collect();
        let t = self.history.len();
        self.last = Some((candle.time, candle.close));

        // 1. Fill the working order against this bar (look-ahead-free). Execution
        //    latency counts down first; then a market order fills at the open and
        //    a resting limit/stop fills only when the bar reaches its level —
        //    otherwise the order keeps working into the next bar.
        if let Some(mut order) = self.pending.take() {
            if order.delay > 0 {
                order.delay -= 1;
                self.pending = Some(order); // still waiting on latency
            } else {
                let ctx = FillCtx {
                    spec: self.spec,
                    candle,
                    history: &self.history,
                    maker: self.maker,
                    taker: self.taker,
                    slip: self.slip,
                    bar: t,
                };
                let keep_working = match &order.action {
                    Action::Enter { side, trigger } => {
                        let side = *side;
                        // A resting limit fill provides liquidity → maker fee.
                        let maker_fill = matches!(trigger, Some((_, LevelKind::Limit)));
                        let level = match trigger {
                            None => Some(candle.open),
                            Some((trig, kind)) => level_fill(side, *trig, *kind, candle),
                        };
                        match level {
                            Some(px) => {
                                execute_entry(
                                    side,
                                    px,
                                    maker_fill,
                                    &ctx,
                                    &mut self.pf,
                                    &mut self.entry_bar,
                                    &mut self.extreme,
                                )?;
                                false
                            }
                            None => true, // level not reached; the order keeps working
                        }
                    }
                    Action::Exit(reason) => {
                        execute_exit(reason, candle.open, &ctx, &mut self.pf, &mut self.entry_bar);
                        false
                    }
                };
                if keep_working {
                    self.pending = Some(order);
                }
            }
        }

        // 2. Update indicators and record the bar.
        let mut values = BTreeMap::new();
        for (name, ind) in &mut self.indicators {
            let input = BarInput {
                candle,
                reference,
                deriv,
                orderbook: orderbook.as_ref(),
                trades: &trades,
            };
            if let Some(v) = ind.update(&input) {
                values.insert(name.clone(), v);
                for (field, fv) in ind.fields() {
                    values.insert(format!("{name}.{field}"), fv);
                }
            }
        }
        self.history.push(BarRow {
            candle: *candle,
            values,
        });
        let idx = self.history.len() - 1;

        // 3. Intrabar stop-loss / take-profit / trailing-stop against this bar's OHLC.
        if self.pf.in_position() {
            // Extend the favourable extreme with this bar before checking the trail.
            self.extreme = if self.pf.is_long() {
                self.extreme.max(candle.high)
            } else {
                self.extreme.min(candle.low)
            };
            if let Some((price, reason)) = intrabar_exit(
                candle,
                &self.spec.risk,
                self.pf.entry_price,
                self.extreme,
                self.pf.is_long(),
            ) {
                let fee = self.pf.qty.abs() * price * self.taker;
                self.pf.exit(price, candle.time, fee, reason);
                self.entry_bar = None;
            } else if self.spec.risk.liquidation {
                // Bankruptcy price: account equity (cash + qty * price) reaches 0.
                let p_liq = -self.pf.cash / self.pf.qty;
                let breached = if self.pf.is_long() {
                    candle.low <= p_liq
                } else {
                    candle.high >= p_liq
                };
                if p_liq > 0.0 && breached {
                    let fee = self.pf.qty.abs() * p_liq * self.taker;
                    self.pf.exit(p_liq, candle.time, fee, "liquidation");
                    self.entry_bar = None;
                }
            }
        }

        // 3b. Charge perpetual funding to the open position from the feed.
        if self.spec.costs.funding && self.pf.in_position() {
            if let Some(d) = deriv {
                // Longs (qty > 0) pay when the rate is positive; shorts receive.
                let payment = self.pf.qty * d.mark_price * d.funding_rate;
                self.pf.apply_funding(payment);
            }
        }

        // 4. Mark equity at the close.
        self.equity.push(EquityPoint {
            time: candle.time,
            equity: self.pf.equity(candle.close),
        });

        // 5. Decide the next signal action. Skip warmup.
        if idx < self.warmup {
            return Ok(());
        }
        let bars_since_entry = self.entry_bar.map(|e| (idx - e) as u32);
        let state = RuleState {
            in_position: self.pf.in_position(),
            bars_since_entry,
        };
        // Close-to-close mode fills on this very bar's close; otherwise the order
        // rests and fills on a later bar (the look-ahead-free default).
        let close_fill = matches!(self.spec.execution.fill_timing, FillTiming::Close);

        if self.pf.in_position() {
            let cond = if self.pf.is_long() {
                &self.spec.exit
            } else {
                self.spec.short_exit.as_ref().unwrap_or(&self.spec.exit)
            };
            if eval_condition(cond, &self.history, idx, state) {
                if close_fill {
                    let ctx = FillCtx {
                        spec: self.spec,
                        candle,
                        history: &self.history,
                        maker: self.maker,
                        taker: self.taker,
                        slip: self.slip,
                        bar: t,
                    };
                    execute_exit(
                        "signal",
                        candle.close,
                        &ctx,
                        &mut self.pf,
                        &mut self.entry_bar,
                    );
                } else {
                    self.pending = Some(Pending {
                        action: Action::Exit("signal"),
                        delay: self.spec.execution.latency_bars,
                    });
                }
            }
        } else if self.pending.is_none() {
            // No order working: a new entry signal places one. Its trigger is the
            // signal bar's close shifted by the configured limit/stop offset.
            let entry_fires = eval_condition(&self.spec.entry, &self.history, idx, state);
            let short_fires = !entry_fires
                && self
                    .spec
                    .short_entry
                    .as_ref()
                    .is_some_and(|c| eval_condition(c, &self.history, idx, state));
            let side = if entry_fires {
                Some(Side::Long)
            } else if short_fires {
                Some(Side::Short)
            } else {
                None
            };
            if let Some(side) = side {
                if close_fill {
                    let ctx = FillCtx {
                        spec: self.spec,
                        candle,
                        history: &self.history,
                        maker: self.maker,
                        taker: self.taker,
                        slip: self.slip,
                        bar: t,
                    };
                    execute_entry(
                        side,
                        candle.close,
                        false, // close-to-close fills are market (taker)
                        &ctx,
                        &mut self.pf,
                        &mut self.entry_bar,
                        &mut self.extreme,
                    )?;
                } else {
                    let trigger = entry_trigger(&self.spec.execution, candle.close);
                    self.pending = Some(Pending {
                        action: Action::Enter { side, trigger },
                        delay: self.spec.execution.latency_bars,
                    });
                }
            }
        }
        Ok(())
    }

    /// Close any open position at the last bar's close and produce the report.
    pub fn finish(mut self) -> BacktestReport {
        if self.pf.in_position() {
            if let Some((time, close)) = self.last {
                let fee = self.pf.qty.abs() * close * self.taker;
                self.pf.exit(close, time, fee, "end");
            }
        }
        let series: Vec<f64> = self.equity.iter().map(|e| e.equity).collect();
        let metrics = metrics::compute(self.capital, &series, &self.pf.trades);
        BacktestReport {
            schema_version: REPORT_SCHEMA_VERSION,
            metrics,
            trades: self.pf.trades,
            equity: self.equity,
            fees_paid: self.pf.fees_paid,
            initial_capital: self.capital,
        }
    }
}

/// Base (unsigned) quantity for the sizing model.
///
/// `equity` is the account equity at entry (the position is opened from flat, so
/// equity equals cash). The resulting notional is capped by the leverage and
/// position limits: without `risk.max_leverage` the cap is 1x equity — no
/// leverage by default — so an order can never exceed what the account can fund.
fn size(
    sizing: Sizing,
    risk: &Risk,
    equity: f64,
    price: f64,
    realized_vol: Option<f64>,
) -> Result<Option<f64>> {
    if price <= 0.0 || equity <= 0.0 {
        return Ok(None);
    }
    let qty = match sizing {
        Sizing::FixedFraction { fraction } => (equity * fraction) / price,
        Sizing::FixedCash { cash: notional } => notional / price,
        Sizing::FixedQty { qty } => qty,
        Sizing::RiskPerTrade { risk_pct } => {
            // Size so a stop-loss hit loses `risk_pct` of equity: the per-unit
            // loss is `price * stop_loss_pct`, so qty = risk_cash / per-unit loss.
            let stop = risk.stop_loss_pct.ok_or_else(|| {
                BacktestError::InvalidSpec(
                    "risk_per_trade sizing requires risk.stop_loss_pct".into(),
                )
            })?;
            if stop <= 0.0 {
                return Ok(None);
            }
            (equity * risk_pct / 100.0) / (price * stop / 100.0)
        }
        Sizing::VolTarget { target_vol, .. } => {
            // Scale notional so the position's per-bar return vol ~= target_vol.
            // No realized vol yet (warming up) => no position this bar.
            let Some(rv) = realized_vol else {
                return Ok(None);
            };
            (equity * target_vol / rv) / price
        }
    };
    if qty <= 0.0 {
        return Ok(None);
    }
    // Cap the notional by the leverage and position limits.
    let max_leverage = risk.max_leverage.unwrap_or(1.0);
    let mut max_notional = equity * max_leverage;
    if let Some(max_pct) = risk.max_position_pct {
        max_notional = max_notional.min(equity * max_pct / 100.0);
    }
    let capped = (qty * price).min(max_notional) / price;
    Ok(Some(capped))
}

/// Intrabar stop-loss / trailing-stop / take-profit fill against the bar's OHLC.
///
/// `extreme` is the most favourable price reached since entry (peak for a long,
/// trough for a short), the trailing-stop reference. Conservative: when a bar's
/// range brackets several levels, the stop (then the trailing stop) is assumed
/// to fill before the target. Levels are side-aware (a short's stop is above
/// entry, its target below).
fn intrabar_exit(
    candle: &Candle,
    risk: &Risk,
    entry: f64,
    extreme: f64,
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
        if let Some(p) = risk.trailing_stop_pct {
            let level = extreme * (1.0 - p / 100.0);
            if candle.low <= level {
                return Some((level, "trailing_stop"));
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
        if let Some(p) = risk.trailing_stop_pct {
            let level = extreme * (1.0 + p / 100.0);
            if candle.high >= level {
                return Some((level, "trailing_stop"));
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

    /// A generated indicator — one that was never in the original hand-written
    /// registry — drives a full backtest, proving the expanded registry
    /// integrates end to end through the engine.
    #[test]
    fn generated_indicator_drives_backtest() {
        // `Alma` is one of the generated scalar (`Input = f64`) indicators.
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h",
                "indicators":{"a":{"type":"Alma","params":[9,0.85,6.0]}},
                "entry":{"cross_above":[{"price":"close"},"a"]},
                "exit":{"cross_below":[{"price":"close"},"a"]},
                "sizing":{"type":"fixed_qty","qty":1}}"#,
        )
        .unwrap();
        let candles: Vec<Candle> = (0..60)
            .map(|i| {
                let px = 100.0 + ((i as f64) * 0.4).sin() * 6.0;
                bar(i, px, px + 0.5, px - 0.5, px)
            })
            .collect();
        let r = run(&spec, &candles).unwrap();
        // It ran over every bar and produced a full equity curve.
        assert_eq!(r.equity.len(), candles.len());
        // The oscillating series crosses the moving average, so it trades.
        assert!(r.metrics.num_trades >= 1);
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

    /// A long trailing stop exits when price retraces past the trailed peak.
    #[test]
    fn trailing_stop() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},
                "exit":{"lt":[{"price":"close"},0]},
                "sizing":{"type":"fixed_qty","qty":1},
                "risk":{"trailing_stop_pct":10.0}}"#,
        )
        .unwrap();
        let candles = [
            bar(0, 100.0, 100.0, 100.0, 100.0), // enter signal
            bar(1, 100.0, 100.0, 100.0, 100.0), // fill enter @ 100
            bar(2, 100.0, 120.0, 119.0, 120.0), // peak 120 (trail 108, low 119 -> no exit)
            bar(3, 118.0, 118.0, 105.0, 106.0), // low 105 <= 108 -> trailing fills @ 108
        ];
        let r = run_with_capital(&spec, &candles, 1000.0).unwrap();
        assert_eq!(r.trades.len(), 1);
        let t = &r.trades[0];
        assert_eq!(t.reason, "trailing_stop");
        assert!((t.exit_price - 108.0).abs() < 1e-9);
        assert!((t.pnl - 8.0).abs() < 1e-9);
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

    /// A multi-output indicator referenced by field (`bb.upper` / `bb.lower`)
    /// resolves end to end through the engine.
    #[test]
    fn multi_output_field_ref_runs() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h",
                "indicators":{"bb":{"type":"Bollinger","params":[5,2]}},
                "entry":{"gt":[{"price":"close"},"bb.upper"]},
                "exit":{"lt":[{"price":"close"},"bb.lower"]},
                "sizing":{"type":"fixed_fraction","fraction":0.5}}"#,
        )
        .unwrap();
        let candles: Vec<Candle> = (0..30)
            .map(|i| {
                let p = 100.0 + (i as f64 * 0.5).sin() * 5.0;
                bar(i, p, p + 1.0, p - 1.0, p)
            })
            .collect();
        let r = run(&spec, &candles).unwrap();
        assert_eq!(r.equity.len(), 30);
    }

    #[test]
    fn vol_target_sizes_inversely_to_vol() {
        // target 1% per bar, realized 2% => notional 0.5x equity => 50 units.
        let q = size(
            Sizing::VolTarget {
                target_vol: 0.01,
                lookback: 5,
            },
            &Risk::default(),
            10_000.0,
            100.0,
            Some(0.02),
        )
        .unwrap()
        .unwrap();
        assert!((q - 50.0).abs() < 1e-9);
    }

    #[test]
    fn vol_target_takes_no_position_without_history() {
        let none = size(
            Sizing::VolTarget {
                target_vol: 0.01,
                lookback: 5,
            },
            &Risk::default(),
            10_000.0,
            100.0,
            None,
        )
        .unwrap();
        assert!(none.is_none());
    }

    #[test]
    fn vol_target_trades_after_warmup() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},
                "exit":{"in_position":false},
                "sizing":{"type":"vol_target","target_vol":0.02,"lookback":3}}"#,
        )
        .unwrap();
        let closes = [100.0, 101.0, 102.0, 101.0, 103.0, 102.0];
        let candles: Vec<Candle> = closes
            .iter()
            .enumerate()
            .map(|(i, &c)| bar(i64::try_from(i).unwrap(), c, c + 0.5, c - 0.5, c))
            .collect();
        let r = run(&spec, &candles).unwrap();
        // Once `lookback` bars of history exist, a vol-targeted position is taken.
        assert!(!r.trades.is_empty());
        assert!(r.trades[0].qty > 0.0);
    }

    #[test]
    fn risk_per_trade_sizes_from_stop() {
        // equity 10_000, risk 1% = 100 cash; stop 2% of price 100 = 2 per unit
        // => 50 units (notional 5_000, under the 1x cap).
        let risk = Risk {
            stop_loss_pct: Some(2.0),
            ..Default::default()
        };
        let q = size(
            Sizing::RiskPerTrade { risk_pct: 1.0 },
            &risk,
            10_000.0,
            100.0,
            None,
        )
        .unwrap()
        .unwrap();
        assert!((q - 50.0).abs() < 1e-9);
    }

    #[test]
    fn risk_per_trade_requires_stop() {
        assert!(size(
            Sizing::RiskPerTrade { risk_pct: 1.0 },
            &Risk::default(),
            10_000.0,
            100.0,
            None
        )
        .is_err());
    }

    #[test]
    fn default_leverage_caps_at_equity() {
        // fixed_cash 50_000 but equity 10_000 and no max_leverage => capped to 1x.
        let q = size(
            Sizing::FixedCash { cash: 50_000.0 },
            &Risk::default(),
            10_000.0,
            100.0,
            None,
        )
        .unwrap()
        .unwrap();
        assert!((q - 100.0).abs() < 1e-9);
    }

    #[test]
    fn max_leverage_allows_more_than_equity() {
        let risk = Risk {
            max_leverage: Some(3.0),
            ..Default::default()
        };
        let q = size(
            Sizing::FixedCash { cash: 50_000.0 },
            &risk,
            10_000.0,
            100.0,
            None,
        )
        .unwrap()
        .unwrap();
        assert!((q - 300.0).abs() < 1e-9); // 3x equity / price
    }

    #[test]
    fn max_position_pct_caps_notional() {
        let risk = Risk {
            max_leverage: Some(5.0),
            max_position_pct: Some(20.0),
            ..Default::default()
        };
        // 5x would allow 50_000, but 20% of equity = 2_000 notional => 20 units.
        let q = size(
            Sizing::FixedCash { cash: 50_000.0 },
            &risk,
            10_000.0,
            100.0,
            None,
        )
        .unwrap()
        .unwrap();
        assert!((q - 20.0).abs() < 1e-9);
    }

    #[test]
    fn leverage_flows_through_run() {
        let candles = [
            bar(0, 100.0, 100.0, 100.0, 100.0),
            bar(1, 100.0, 100.0, 100.0, 100.0), // enter @ open 100
            bar(2, 100.0, 100.0, 100.0, 100.0),
        ];
        let no_lev = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},
                "exit":{"in_position":false},
                "sizing":{"type":"fixed_cash","cash":50000}}"#,
        )
        .unwrap();
        let r0 = run_with_capital(&no_lev, &candles, 10_000.0).unwrap();
        assert!((r0.trades[0].qty - 100.0).abs() < 1e-9); // capped to 1x equity

        let levered = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},
                "exit":{"in_position":false},
                "sizing":{"type":"fixed_cash","cash":50000},
                "risk":{"max_leverage":3}}"#,
        )
        .unwrap();
        let r1 = run_with_capital(&levered, &candles, 10_000.0).unwrap();
        assert!((r1.trades[0].qty - 300.0).abs() < 1e-9); // 3x equity
    }

    #[test]
    fn limit_entry_fills_on_dip() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},
                "exit":{"in_position":false},
                "sizing":{"type":"fixed_qty","qty":1},
                "execution":{"order_type":"limit","limit_offset_pct":-1.0}}"#,
        )
        .unwrap();
        let candles = [
            bar(0, 100.0, 100.0, 100.0, 100.0), // signal -> limit works @ 99
            bar(1, 100.0, 101.0, 100.0, 100.0), // low 100 > 99: no fill, keeps working
            bar(2, 100.0, 100.0, 98.0, 99.0),   // low 98 <= 99: fills @ 99
        ];
        let r = run_with_capital(&spec, &candles, 10_000.0).unwrap();
        assert_eq!(r.trades.len(), 1);
        assert!((r.trades[0].entry_price - 99.0).abs() < 1e-9);
    }

    #[test]
    fn limit_entry_never_fills_without_a_dip() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},
                "exit":{"in_position":false},
                "sizing":{"type":"fixed_qty","qty":1},
                "execution":{"order_type":"limit","limit_offset_pct":-1.0}}"#,
        )
        .unwrap();
        let candles = [
            bar(0, 100.0, 100.0, 100.0, 100.0),
            bar(1, 100.0, 101.0, 100.0, 100.0),
            bar(2, 100.0, 102.0, 100.0, 101.0), // low never reaches 99
        ];
        let r = run_with_capital(&spec, &candles, 10_000.0).unwrap();
        assert!(r.trades.is_empty());
    }

    #[test]
    fn stop_entry_fills_on_breakout() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},
                "exit":{"in_position":false},
                "sizing":{"type":"fixed_qty","qty":1},
                "execution":{"order_type":"stop","stop_offset_pct":1.0}}"#,
        )
        .unwrap();
        let candles = [
            bar(0, 100.0, 100.0, 100.0, 100.0), // signal -> stop works @ 101
            bar(1, 100.0, 100.5, 100.0, 100.0), // high 100.5 < 101: no fill
            bar(2, 100.0, 102.0, 100.0, 101.0), // high 102 >= 101: fills @ 101
        ];
        let r = run_with_capital(&spec, &candles, 10_000.0).unwrap();
        assert_eq!(r.trades.len(), 1);
        assert!((r.trades[0].entry_price - 101.0).abs() < 1e-9);
    }

    #[test]
    fn limit_order_requires_offset() {
        // `parse` validates, so an order_type without its offset is rejected up front.
        assert!(StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},"exit":{"in_position":true},
                "sizing":{"type":"fixed_qty","qty":1},
                "execution":{"order_type":"limit"}}"#,
        )
        .is_err());
    }

    #[test]
    fn stop_limit_is_unsupported() {
        assert!(StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},"exit":{"in_position":true},
                "sizing":{"type":"fixed_qty","qty":1},
                "execution":{"order_type":"stop_limit"}}"#,
        )
        .is_err());
    }

    #[test]
    fn latency_delays_the_fill() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},
                "exit":{"in_position":false},
                "sizing":{"type":"fixed_qty","qty":1},
                "execution":{"latency_bars":1}}"#,
        )
        .unwrap();
        let candles = [
            bar(0, 100.0, 100.0, 100.0, 100.0), // signal at close
            bar(1, 110.0, 110.0, 110.0, 110.0), // would fill here without latency
            bar(2, 120.0, 120.0, 120.0, 120.0), // fills here after 1 bar of latency
        ];
        let r = run_with_capital(&spec, &candles, 10_000.0).unwrap();
        assert_eq!(r.trades.len(), 1);
        assert!((r.trades[0].entry_price - 120.0).abs() < 1e-9);
    }

    #[test]
    fn partial_fills_cap_entry_to_participation() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},
                "exit":{"in_position":false},
                "sizing":{"type":"fixed_qty","qty":100},
                "execution":{"partial_fills":true,"max_participation":0.05}}"#,
        )
        .unwrap();
        // The fill bar's volume is 1000, so the cap is 0.05 * 1000 = 50 units,
        // below the desired 100.
        let vbar = |time, volume| Candle {
            time,
            open: 100.0,
            high: 100.0,
            low: 100.0,
            close: 100.0,
            volume,
        };
        let candles = [vbar(0, 0.0), vbar(1, 1000.0), vbar(2, 1000.0)];
        let r = run_with_capital(&spec, &candles, 1_000_000.0).unwrap();
        assert_eq!(r.trades.len(), 1);
        assert!((r.trades[0].qty - 50.0).abs() < 1e-9);
    }

    #[test]
    fn partial_fills_requires_participation() {
        assert!(StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},"exit":{"in_position":true},
                "sizing":{"type":"fixed_qty","qty":1},
                "execution":{"partial_fills":true}}"#,
        )
        .is_err());
    }

    #[test]
    fn fill_timing_close_fills_same_bar() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},100]},
                "exit":{"lt":[{"price":"close"},100]},
                "sizing":{"type":"fixed_qty","qty":1},
                "execution":{"fill_timing":"close"}}"#,
        )
        .unwrap();
        let candles = [
            bar(0, 90.0, 90.0, 90.0, 90.0),   // close 90: no entry
            bar(1, 95.0, 105.0, 95.0, 101.0), // close 101 > 100: entry @ close 101
            bar(2, 100.0, 100.0, 90.0, 99.0), // close 99 < 100: exit @ close 99
        ];
        let r = run_with_capital(&spec, &candles, 10_000.0).unwrap();
        assert_eq!(r.trades.len(), 1);
        assert!((r.trades[0].entry_price - 101.0).abs() < 1e-9); // same-bar close
        assert!((r.trades[0].exit_price - 99.0).abs() < 1e-9);
    }

    #[test]
    fn fill_timing_close_rejects_limit_and_latency() {
        // Close fills can't express the next-bar limit/stop or latency models.
        assert!(StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},"exit":{"in_position":true},
                "sizing":{"type":"fixed_qty","qty":1},
                "execution":{"fill_timing":"close","order_type":"limit","limit_offset_pct":-1.0}}"#,
        )
        .is_err());
        assert!(StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},"exit":{"in_position":true},
                "sizing":{"type":"fixed_qty","qty":1},
                "execution":{"fill_timing":"close","latency_bars":1}}"#,
        )
        .is_err());
    }

    #[test]
    fn streaming_matches_batch() {
        // Feeding bars one at a time through the public streaming API produces
        // the same report as the batch runner — backtest and live are one path.
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h",
                "indicators":{"f":{"type":"Ema","params":[3]}},
                "entry":{"cross_above":[{"price":"close"},"f"]},
                "exit":{"cross_below":[{"price":"close"},"f"]},
                "sizing":{"type":"fixed_qty","qty":1}}"#,
        )
        .unwrap();
        let candles: Vec<Candle> = (0..30i64)
            .map(|i| {
                let px = 100.0 + ((i as f64) * 0.5).sin() * 5.0;
                bar(i, px, px + 0.5, px - 0.5, px)
            })
            .collect();

        let batch = run_with_capital(&spec, &candles, 10_000.0).unwrap();

        let mut bt = StreamingBacktest::new(&spec, 10_000.0).unwrap();
        for c in &candles {
            bt.step(c).unwrap();
        }
        let streamed = bt.finish();

        assert!(batch.metrics.num_trades >= 1);
        assert_eq!(batch.metrics.num_trades, streamed.metrics.num_trades);
        assert_eq!(batch.trades.len(), streamed.trades.len());
        assert_eq!(batch.equity.len(), streamed.equity.len());
        assert!(
            (batch.equity.last().unwrap().equity - streamed.equity.last().unwrap().equity).abs()
                < 1e-12
        );
    }

    #[test]
    fn pairwise_indicator_uses_reference_series() {
        // A pairwise indicator (Pearson correlation) is fed the reference
        // series' close as its second input via run_with_ref.
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h",
                "indicators":{"c":{"type":"PearsonCorrelation","params":[3]}},
                "entry":{"gt":["c",0.5]},
                "exit":{"lt":["c",-2.0]},
                "sizing":{"type":"fixed_qty","qty":1}}"#,
        )
        .unwrap();
        let primary: Vec<Candle> = [100.0, 101.0, 102.0, 101.0, 103.0, 102.0, 104.0, 103.0]
            .iter()
            .zip(0i64..)
            .map(|(&c, i)| bar(i, c, c + 0.5, c - 0.5, c))
            .collect();
        // A perfectly correlated reference series → correlation ~1 > 0.5 → entry.
        let reference: Vec<Candle> = [50.0, 50.5, 51.0, 50.5, 51.5, 51.0, 52.0, 51.5]
            .iter()
            .zip(0i64..)
            .map(|(&c, i)| bar(i, c, c + 0.2, c - 0.2, c))
            .collect();

        let with_ref = run_with_ref(&spec, &primary, &reference, 10_000.0).unwrap();
        assert!(with_ref.metrics.num_trades >= 1);

        // Without a reference series the pairwise indicator yields nothing, so
        // the entry condition never fires.
        let without_ref = run_with_capital(&spec, &primary, 10_000.0).unwrap();
        assert_eq!(without_ref.metrics.num_trades, 0);
    }

    #[test]
    fn pairwise_multi_output_exposes_fields() {
        // A pairwise multi-output indicator exposes its named fields when fed a
        // reference value.
        let mut ind = registry::build("RelativeStrengthAB", &[3.0, 3.0]).unwrap();
        let mut names: Vec<&str> = Vec::new();
        let prices = [
            100.0, 102.0, 104.0, 103.0, 105.0, 106.0, 107.0, 108.0, 109.0, 110.0,
        ];
        for (i, &px) in prices.iter().enumerate() {
            let c = Candle {
                time: i64::try_from(i).unwrap(),
                open: px,
                high: px,
                low: px,
                close: px,
                volume: 0.0,
            };
            let input = BarInput {
                candle: &c,
                reference: Some(px * 0.9),
                deriv: None,
                orderbook: None,
                trades: &[],
            };
            if ind.update(&input).is_some() {
                names = ind.fields().iter().map(|(n, _)| *n).collect();
            }
        }
        assert!(names.contains(&"ratio"), "fields: {names:?}");
    }

    #[test]
    fn run_with_ref_rejects_length_mismatch() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},"exit":{"in_position":true},
                "sizing":{"type":"fixed_qty","qty":1}}"#,
        )
        .unwrap();
        let a = [bar(0, 1.0, 1.0, 1.0, 1.0), bar(1, 1.0, 1.0, 1.0, 1.0)];
        let b = [bar(0, 1.0, 1.0, 1.0, 1.0)];
        assert!(run_with_ref(&spec, &a, &b, 10_000.0).is_err());
    }

    fn sample_tick(funding_rate: f64) -> DerivativesTick {
        DerivativesTick {
            funding_rate,
            mark_price: 100.0,
            index_price: 100.0,
            futures_price: 100.0,
            open_interest: 1000.0,
            long_size: 600.0,
            short_size: 400.0,
            taker_buy_volume: 50.0,
            taker_sell_volume: 40.0,
            long_liquidation: 0.0,
            short_liquidation: 0.0,
            timestamp: 0,
        }
    }

    #[test]
    fn derivatives_indicator_uses_feed() {
        // FundingRate passes the tick's funding rate through; the feed drives it.
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h",
                "indicators":{"f":{"type":"FundingRate","params":[]}},
                "entry":{"gt":["f",0.0]},
                "exit":{"lt":["f",-1.0]},
                "sizing":{"type":"fixed_qty","qty":1}}"#,
        )
        .unwrap();
        let candles: Vec<Candle> = (0i64..5)
            .map(|i| bar(i, 100.0, 100.0, 100.0, 100.0))
            .collect();
        let derivs = vec![sample_tick(0.01); 5];

        let with_feed = run_with_deriv(&spec, &candles, &derivs, 10_000.0).unwrap();
        assert!(with_feed.metrics.num_trades >= 1);

        // Without a derivatives feed the indicator yields nothing → no entry.
        let without_feed = run_with_capital(&spec, &candles, 10_000.0).unwrap();
        assert_eq!(without_feed.metrics.num_trades, 0);
    }

    #[test]
    fn order_book_indicator_uses_feed() {
        use crate::data::Level;
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h",
                "indicators":{"i":{"type":"OrderBookImbalanceTop1","params":[]}},
                "entry":{"gt":["i",0.0]},
                "exit":{"lt":["i",-2.0]},
                "sizing":{"type":"fixed_qty","qty":1}}"#,
        )
        .unwrap();
        let candles: Vec<Candle> = (0i64..5)
            .map(|t| bar(t, 100.0, 100.0, 100.0, 100.0))
            .collect();
        // A bid-heavy book → positive top-of-book imbalance → entry.
        let book = OrderBook {
            bids: vec![Level {
                price: 100.0,
                size: 9.0,
            }],
            asks: vec![Level {
                price: 101.0,
                size: 1.0,
            }],
        };
        let books = vec![book; 5];

        let with_feed = run_with_orderbook(&spec, &candles, &books, 10_000.0).unwrap();
        assert!(with_feed.metrics.num_trades >= 1);

        let without_feed = run_with_capital(&spec, &candles, 10_000.0).unwrap();
        assert_eq!(without_feed.metrics.num_trades, 0);
    }

    #[test]
    fn trade_indicator_replays_bar_trades() {
        use crate::data::{TradePrint, TradeSide};
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h",
                "indicators":{"cvd":{"type":"CumulativeVolumeDelta","params":[]}},
                "entry":{"gt":["cvd",0.0]},
                "exit":{"lt":["cvd",-1.0]},
                "sizing":{"type":"fixed_qty","qty":1}}"#,
        )
        .unwrap();
        let candles: Vec<Candle> = (0i64..5)
            .map(|t| bar(t, 100.0, 100.0, 100.0, 100.0))
            .collect();
        let buy = TradePrint {
            price: 100.0,
            size: 5.0,
            side: TradeSide::Buy,
            timestamp: 0,
        };
        // Two buy trades per bar → cumulative volume delta grows positive.
        let trades: Vec<Vec<TradePrint>> = (0..5).map(|_| vec![buy, buy]).collect();

        let with_feed = run_with_trades(&spec, &candles, &trades, 10_000.0).unwrap();
        assert!(with_feed.metrics.num_trades >= 1);

        let without_feed = run_with_capital(&spec, &candles, 10_000.0).unwrap();
        assert_eq!(without_feed.metrics.num_trades, 0);
    }

    #[test]
    fn funding_charges_an_open_long() {
        let with_funding = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},
                "exit":{"in_position":false},
                "sizing":{"type":"fixed_qty","qty":1},
                "costs":{"funding":true}}"#,
        )
        .unwrap();
        let candles = [
            bar(0, 100.0, 100.0, 100.0, 100.0),
            bar(1, 100.0, 100.0, 100.0, 100.0),
            bar(2, 100.0, 100.0, 100.0, 100.0),
        ];
        let derivs = vec![sample_tick(0.01); 3]; // funding 1% of mark 100 = 1.0/bar

        let funded = run_with_deriv(&with_funding, &candles, &derivs, 10_000.0).unwrap();
        assert!(funded.fees_paid > 0.0); // a long paid funding

        let no_funding = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},
                "exit":{"in_position":false},
                "sizing":{"type":"fixed_qty","qty":1}}"#,
        )
        .unwrap();
        let unfunded = run_with_deriv(&no_funding, &candles, &derivs, 10_000.0).unwrap();
        assert!(funded.equity.last().unwrap().equity < unfunded.equity.last().unwrap().equity);
    }

    #[test]
    fn leverage_liquidation_closes_at_bankruptcy() {
        // 5x long: capital 1000, notional 5000 → qty 50, cash -4000, bankruptcy
        // price -(-4000)/50 = 80.
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},
                "exit":{"in_position":false},
                "sizing":{"type":"fixed_cash","cash":5000},
                "risk":{"max_leverage":5,"liquidation":true}}"#,
        )
        .unwrap();
        let candles = [
            bar(0, 100.0, 100.0, 100.0, 100.0), // signal
            bar(1, 100.0, 100.0, 95.0, 98.0),   // enter @ open 100; low 95 > 80: safe
            bar(2, 90.0, 90.0, 70.0, 75.0),     // low 70 <= 80: liquidate @ 80
        ];
        let r = run_with_capital(&spec, &candles, 1000.0).unwrap();
        assert_eq!(r.trades.len(), 1);
        assert_eq!(r.trades[0].reason, "liquidation");
        assert!((r.trades[0].exit_price - 80.0).abs() < 1e-9);
        assert!(r.equity.last().unwrap().equity.abs() < 1e-6); // account wiped out
    }

    #[test]
    fn limit_entry_pays_maker_fee() {
        let candles = [
            bar(0, 100.0, 100.0, 100.0, 100.0),
            bar(1, 100.0, 100.0, 100.0, 100.0),
            bar(2, 100.0, 100.0, 100.0, 100.0),
        ];
        // A market entry pays the taker fee; a resting limit entry pays maker.
        let market = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},"exit":{"in_position":false},
                "sizing":{"type":"fixed_qty","qty":1},
                "costs":{"maker_bps":0,"taker_bps":200}}"#,
        )
        .unwrap();
        let limit = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},"exit":{"in_position":false},
                "sizing":{"type":"fixed_qty","qty":1},
                "costs":{"maker_bps":0,"taker_bps":200},
                "execution":{"order_type":"limit","limit_offset_pct":0.0}}"#,
        )
        .unwrap();
        let market_fees = run_with_capital(&market, &candles, 10_000.0)
            .unwrap()
            .fees_paid;
        let limit_fees = run_with_capital(&limit, &candles, 10_000.0)
            .unwrap()
            .fees_paid;
        assert!(limit_fees < market_fees); // maker (0) saved versus taker on entry
    }

    #[test]
    fn run_with_deriv_rejects_length_mismatch() {
        let spec = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},0]},"exit":{"in_position":true},
                "sizing":{"type":"fixed_qty","qty":1}}"#,
        )
        .unwrap();
        let candles = [bar(0, 1.0, 1.0, 1.0, 1.0), bar(1, 1.0, 1.0, 1.0, 1.0)];
        let derivs = [sample_tick(0.0)];
        assert!(run_with_deriv(&spec, &candles, &derivs, 10_000.0).is_err());
    }
}
