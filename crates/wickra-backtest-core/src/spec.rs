//! The data-driven strategy specification (`StrategySpec`).
//!
//! A strategy is **data, not code** — a JSON document — so the exact same
//! strategy runs identically across every Wickra language binding and over the
//! C-ABI. This module defines the serde representation of the spec and a
//! structural [`StrategySpec::validate`] that checks every indicator reference
//! is declared.
//!
//! See `schema/strategy_spec.schema.json` (generated) for the canonical schema.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::error::{BacktestError, Result};

/// Current strategy-spec format version. Bumped on breaking DSL changes.
pub const SPEC_VERSION: u32 = 1;

fn default_spec_version() -> u32 {
    SPEC_VERSION
}

/// A complete strategy specification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StrategySpec {
    /// Spec format version (defaults to [`SPEC_VERSION`]).
    #[serde(default = "default_spec_version")]
    pub spec_version: u32,
    /// Primary trading symbol.
    pub symbol: String,
    /// Optional reference symbol for pair indicators.
    #[serde(default)]
    pub ref_symbol: Option<String>,
    /// Bar timeframe (e.g. `"1h"`).
    pub timeframe: String,
    /// Named indicators available to the rules.
    pub indicators: BTreeMap<String, IndicatorSpec>,
    /// Long-entry condition.
    pub entry: Condition,
    /// Long-exit condition.
    pub exit: Condition,
    /// Optional short-entry condition.
    #[serde(default)]
    pub short_entry: Option<Condition>,
    /// Optional short-exit condition.
    #[serde(default)]
    pub short_exit: Option<Condition>,
    /// Position sizing.
    pub sizing: Sizing,
    /// Trading costs.
    #[serde(default)]
    pub costs: Costs,
    /// Risk controls.
    #[serde(default)]
    pub risk: Risk,
    /// Execution model.
    #[serde(default)]
    pub execution: Execution,
    /// Explicit warmup bars (defaults to the max indicator warmup).
    #[serde(default)]
    pub warmup: Option<u32>,
}

impl StrategySpec {
    /// Parse a spec from JSON and validate it.
    pub fn parse(json: &str) -> Result<Self> {
        let spec: Self =
            serde_json::from_str(json).map_err(|e| BacktestError::InvalidSpec(e.to_string()))?;
        spec.validate()?;
        Ok(spec)
    }

    /// Validate structural invariants: every indicator referenced by the rules
    /// must be declared in `indicators`.
    pub fn validate(&self) -> Result<()> {
        let declared: BTreeSet<&str> = self.indicators.keys().map(String::as_str).collect();
        check_condition(&self.entry, &declared)?;
        check_condition(&self.exit, &declared)?;
        if let Some(c) = &self.short_entry {
            check_condition(c, &declared)?;
        }
        if let Some(c) = &self.short_exit {
            check_condition(c, &declared)?;
        }
        match self.execution.order_type {
            OrderType::Limit if self.execution.limit_offset_pct.is_none() => {
                return Err(BacktestError::InvalidSpec(
                    "limit order_type requires execution.limit_offset_pct".into(),
                ));
            }
            OrderType::Stop if self.execution.stop_offset_pct.is_none() => {
                return Err(BacktestError::InvalidSpec(
                    "stop order_type requires execution.stop_offset_pct".into(),
                ));
            }
            OrderType::StopLimit => {
                return Err(BacktestError::InvalidSpec(
                    "stop_limit order_type is not supported yet".into(),
                ));
            }
            _ => {}
        }
        if self.execution.partial_fills && self.execution.max_participation.is_none() {
            return Err(BacktestError::InvalidSpec(
                "partial_fills requires execution.max_participation".into(),
            ));
        }
        if matches!(self.execution.fill_timing, FillTiming::Close) {
            // Close fills happen on the signalling bar itself, which the resting
            // limit/stop and latency models (both next-bar) cannot express.
            if !matches!(self.execution.order_type, OrderType::Market) {
                return Err(BacktestError::InvalidSpec(
                    "fill_timing close requires a market order_type".into(),
                ));
            }
            if self.execution.latency_bars != 0 {
                return Err(BacktestError::InvalidSpec(
                    "fill_timing close is incompatible with latency_bars".into(),
                ));
            }
        }
        Ok(())
    }
}

/// One indicator instance: a `wickra-core` type name plus its parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndicatorSpec {
    /// The `wickra-core` indicator type name (e.g. `"Ema"`).
    #[serde(rename = "type")]
    pub kind: String,
    /// Constructor parameters.
    #[serde(default)]
    pub params: Vec<f64>,
    /// Which feed drives it.
    #[serde(default)]
    pub feed: Feed,
}

/// The data feed an indicator is driven by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feed {
    /// OHLCV candles (default).
    #[default]
    Kline,
    /// Trade prints.
    Trade,
    /// Order-book snapshots.
    Orderbook,
}

/// A price field of the current bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceField {
    /// Open.
    Open,
    /// High.
    High,
    /// Low.
    Low,
    /// Close.
    Close,
    /// Volume.
    Volume,
    /// `(high + low + close) / 3`.
    Hlc3,
    /// `(open + high + low + close) / 4`.
    Ohlc4,
}

/// A value node — evaluates to a number each bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Operand {
    /// Indicator reference by name, optionally `"name.field"` for multi-output.
    Ref(String),
    /// A literal constant.
    Const(f64),
    /// A compound expression.
    Expr(Box<OperandExpr>),
}

/// The object-shaped operand forms.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperandExpr {
    /// A price field of the current bar.
    Price(PriceField),
    /// The value of an operand `n` bars ago: `["operand", n]`.
    Prev((Box<Operand>, u32)),
    /// `a + b`.
    Add((Box<Operand>, Box<Operand>)),
    /// `a - b`.
    Sub((Box<Operand>, Box<Operand>)),
    /// `a * b`.
    Mul((Box<Operand>, Box<Operand>)),
    /// `a / b`.
    Div((Box<Operand>, Box<Operand>)),
}

/// A boolean node — evaluates to true/false each bar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    /// `a > b`.
    Gt((Operand, Operand)),
    /// `a < b`.
    Lt((Operand, Operand)),
    /// `a >= b`.
    Ge((Operand, Operand)),
    /// `a <= b`.
    Le((Operand, Operand)),
    /// `a == b`.
    Eq((Operand, Operand)),
    /// `a != b`.
    Ne((Operand, Operand)),
    /// `a` crosses above `b` this bar.
    CrossAbove((Operand, Operand)),
    /// `a` crosses below `b` this bar.
    CrossBelow((Operand, Operand)),
    /// `lo <= a <= hi`: `[a, lo, hi]`.
    Between((Operand, Operand, Operand)),
    /// `a` is greater than its value `n` bars ago: `[a, n]`.
    Rising((Operand, u32)),
    /// `a` is less than its value `n` bars ago: `[a, n]`.
    Falling((Operand, u32)),
    /// All sub-conditions true (AND).
    All(Vec<Condition>),
    /// Any sub-condition true (OR).
    Any(Vec<Condition>),
    /// Negation.
    Not(Box<Condition>),
    /// True iff a position is currently open.
    InPosition(bool),
    /// Predicate on the number of bars since entry.
    BarsSinceEntry(IntPredicate),
}

/// An integer comparison predicate (used by stateful conditions).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntPredicate {
    /// `> n`.
    Gt(u32),
    /// `< n`.
    Lt(u32),
    /// `>= n`.
    Ge(u32),
    /// `<= n`.
    Le(u32),
    /// `== n`.
    Eq(u32),
}

/// Position sizing model.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Sizing {
    /// A fraction of current equity.
    FixedFraction {
        /// Fraction in `[0, 1]`.
        fraction: f64,
    },
    /// A fixed quantity of the base asset.
    FixedQty {
        /// Quantity.
        qty: f64,
    },
    /// A fixed cash notional.
    FixedCash {
        /// Cash amount.
        cash: f64,
    },
    /// Size to a target volatility: the position notional is scaled so the
    /// position's per-bar return volatility approximates `target_vol`. With
    /// realized per-bar volatility `rv` over `lookback` bars, the notional is
    /// `equity * target_vol / rv` (then capped by the leverage limits). No
    /// position is taken until `lookback` bars of history exist.
    VolTarget {
        /// Target per-bar return volatility, as a fraction (e.g. `0.02` = 2%).
        target_vol: f64,
        /// Lookback bars for the realized-volatility estimate.
        lookback: u32,
    },
    /// Size from the stop-loss distance and a per-trade risk budget.
    RiskPerTrade {
        /// Risk per trade in percent of equity.
        risk_pct: f64,
    },
}

/// Trading costs.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Costs {
    /// Maker fee in basis points.
    #[serde(default)]
    pub maker_bps: f64,
    /// Taker fee in basis points.
    #[serde(default)]
    pub taker_bps: f64,
    /// Slippage model.
    #[serde(default)]
    pub slippage: Slippage,
}

/// Slippage model.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Slippage {
    /// A fixed number of basis points.
    FixedBps {
        /// Basis points.
        bps: f64,
    },
    /// Slippage equal to the bid/ask spread (needs an order-book feed).
    Spread,
    /// Linear price impact in the traded volume.
    VolumeImpact {
        /// Impact coefficient.
        coef: f64,
    },
}

impl Default for Slippage {
    fn default() -> Self {
        Self::FixedBps { bps: 0.0 }
    }
}

/// Risk controls (all optional).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Risk {
    /// Stop-loss as a percent move against the position.
    #[serde(default)]
    pub stop_loss_pct: Option<f64>,
    /// Take-profit as a percent move in favour.
    #[serde(default)]
    pub take_profit_pct: Option<f64>,
    /// Trailing-stop as a percent retrace from the peak.
    #[serde(default)]
    pub trailing_stop_pct: Option<f64>,
    /// Maximum leverage.
    #[serde(default)]
    pub max_leverage: Option<f64>,
    /// Maximum position as a percent of equity.
    #[serde(default)]
    pub max_position_pct: Option<f64>,
}

/// Execution model.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Execution {
    /// Order type.
    #[serde(default)]
    pub order_type: OrderType,
    /// When a signalled order fills.
    #[serde(default)]
    pub fill_timing: FillTiming,
    /// Limit-order trigger as a percent offset from the signal bar's close
    /// (required for `order_type = "limit"`). Negative places a long limit
    /// below the market (buy the dip); positive places a short limit above it.
    #[serde(default)]
    pub limit_offset_pct: Option<f64>,
    /// Stop-order trigger as a percent offset from the signal bar's close
    /// (required for `order_type = "stop"`). Positive places a long stop above
    /// the market (breakout); negative places a short stop below it.
    #[serde(default)]
    pub stop_offset_pct: Option<f64>,
    /// Simulated latency in bars before a fill.
    #[serde(default)]
    pub latency_bars: u32,
    /// Whether partial fills are modelled. When set, an entry fills at most
    /// `max_participation * bar_volume` and the unfilled remainder is cancelled.
    #[serde(default)]
    pub partial_fills: bool,
    /// Maximum fraction of a bar's volume a single entry may consume (required
    /// when `partial_fills` is set).
    #[serde(default)]
    pub max_participation: Option<f64>,
}

/// Order type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderType {
    /// Market order (default).
    #[default]
    Market,
    /// Limit order.
    Limit,
    /// Stop order.
    Stop,
    /// Stop-limit order.
    StopLimit,
}

/// When a signalled order fills.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FillTiming {
    /// On the next bar's open — the look-ahead-bias-free default.
    #[default]
    NextOpen,
    /// On the signalling bar's own close (close-to-close execution). An opt-in,
    /// deliberately optimistic mode: the fill uses the very close that produced
    /// the signal, which is not actually tradeable in live execution. Market
    /// orders only, and incompatible with `latency_bars`.
    Close,
}

// --- validation helpers ------------------------------------------------------

fn check_operand(op: &Operand, declared: &BTreeSet<&str>) -> Result<()> {
    match op {
        Operand::Ref(name) => {
            let base = name.split('.').next().unwrap_or(name.as_str());
            if !declared.contains(base) {
                return Err(BacktestError::UndeclaredRef(name.clone()));
            }
        }
        Operand::Const(_) => {}
        Operand::Expr(expr) => match expr.as_ref() {
            OperandExpr::Price(_) => {}
            OperandExpr::Prev((a, _)) => check_operand(a, declared)?,
            OperandExpr::Add((a, b))
            | OperandExpr::Sub((a, b))
            | OperandExpr::Mul((a, b))
            | OperandExpr::Div((a, b)) => {
                check_operand(a, declared)?;
                check_operand(b, declared)?;
            }
        },
    }
    Ok(())
}

fn check_condition(cond: &Condition, declared: &BTreeSet<&str>) -> Result<()> {
    match cond {
        Condition::Gt((a, b))
        | Condition::Lt((a, b))
        | Condition::Ge((a, b))
        | Condition::Le((a, b))
        | Condition::Eq((a, b))
        | Condition::Ne((a, b))
        | Condition::CrossAbove((a, b))
        | Condition::CrossBelow((a, b)) => {
            check_operand(a, declared)?;
            check_operand(b, declared)?;
        }
        Condition::Between((a, lo, hi)) => {
            check_operand(a, declared)?;
            check_operand(lo, declared)?;
            check_operand(hi, declared)?;
        }
        Condition::Rising((a, _)) | Condition::Falling((a, _)) => check_operand(a, declared)?,
        Condition::All(cs) | Condition::Any(cs) => {
            for c in cs {
                check_condition(c, declared)?;
            }
        }
        Condition::Not(c) => check_condition(c, declared)?,
        Condition::InPosition(_) | Condition::BarsSinceEntry(_) => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"{
      "spec_version": 1, "symbol": "BTCUSDT", "timeframe": "1h",
      "indicators": {
        "ema_fast": {"type": "Ema", "params": [20]},
        "ema_slow": {"type": "Ema", "params": [50]},
        "rsi": {"type": "Rsi", "params": [14]}
      },
      "entry": {"all": [{"cross_above": ["ema_fast", "ema_slow"]}, {"lt": ["rsi", 70]}]},
      "exit": {"any": [{"cross_below": ["ema_fast", "ema_slow"]}, {"gt": ["rsi", 80]}]},
      "sizing": {"type": "fixed_fraction", "fraction": 0.95},
      "costs": {"maker_bps": 2, "taker_bps": 5, "slippage": {"type": "fixed_bps", "bps": 2}},
      "risk": {"stop_loss_pct": 2.0, "take_profit_pct": 5.0},
      "execution": {"order_type": "market", "fill_timing": "next_open"}
    }"#;

    #[test]
    fn parses_and_validates_example() {
        let spec = StrategySpec::parse(EXAMPLE).unwrap();
        assert_eq!(spec.spec_version, 1);
        assert_eq!(spec.symbol, "BTCUSDT");
        assert_eq!(spec.indicators.len(), 3);
        assert!(matches!(spec.sizing, Sizing::FixedFraction { .. }));
        assert!(matches!(spec.execution.fill_timing, FillTiming::NextOpen));
    }

    #[test]
    fn roundtrips_losslessly() {
        let spec = StrategySpec::parse(EXAMPLE).unwrap();
        let json = serde_json::to_string(&spec).unwrap();
        let again: StrategySpec = serde_json::from_str(&json).unwrap();
        assert_eq!(spec, again);
    }

    #[test]
    fn defaults_fill_in() {
        let json = r#"{
          "symbol": "ETHUSDT", "timeframe": "5m",
          "indicators": {"sma": {"type": "Sma", "params": [10]}},
          "entry": {"gt": ["sma", {"price": "close"}]},
          "exit": {"lt": ["sma", {"price": "close"}]},
          "sizing": {"type": "fixed_qty", "qty": 1.0}
        }"#;
        let spec = StrategySpec::parse(json).unwrap();
        assert_eq!(spec.spec_version, SPEC_VERSION);
        assert_eq!(spec.execution.fill_timing, FillTiming::NextOpen);
        assert_eq!(spec.indicators["sma"].feed, Feed::Kline);
        assert!(spec.risk.stop_loss_pct.is_none());
        assert!((spec.costs.maker_bps).abs() < f64::EPSILON);
    }

    #[test]
    fn rejects_undeclared_reference() {
        let json = r#"{
          "symbol": "X", "timeframe": "1h",
          "indicators": {"a": {"type": "Sma", "params": [5]}},
          "entry": {"gt": ["a", "b"]},
          "exit": {"in_position": true},
          "sizing": {"type": "fixed_qty", "qty": 1.0}
        }"#;
        let err = StrategySpec::parse(json).unwrap_err();
        assert!(matches!(err, BacktestError::UndeclaredRef(r) if r == "b"));
    }

    #[test]
    fn operand_forms_parse() {
        let op: Operand = serde_json::from_str(r#""ema_fast""#).unwrap();
        assert!(matches!(op, Operand::Ref(_)));
        let op: Operand = serde_json::from_str("70").unwrap();
        assert!(matches!(op, Operand::Const(_)));
        let op: Operand = serde_json::from_str(r#"{"price": "close"}"#).unwrap();
        assert!(matches!(op, Operand::Expr(_)));
        let op: Operand = serde_json::from_str(r#"{"prev": ["ema_fast", 1]}"#).unwrap();
        assert!(matches!(op, Operand::Expr(_)));
        let op: Operand = serde_json::from_str(r#"{"add": [1, 2]}"#).unwrap();
        assert!(matches!(op, Operand::Expr(_)));
    }

    #[test]
    fn multi_output_ref_is_allowed_when_base_declared() {
        let json = r#"{
          "symbol": "X", "timeframe": "1h",
          "indicators": {"macd": {"type": "Macd", "params": [12, 26, 9]}},
          "entry": {"cross_above": ["macd.macd", "macd.signal"]},
          "exit": {"in_position": true},
          "sizing": {"type": "fixed_qty", "qty": 1.0}
        }"#;
        assert!(StrategySpec::parse(json).is_ok());
    }
}
