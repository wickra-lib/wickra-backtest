//! Evaluation of the strategy DSL ([`Operand`] / [`Condition`]) against the
//! per-bar history. Operands resolve to a number (or `None` when an indicator
//! is still warming up); conditions resolve to a boolean (false when any
//! operand is missing). Cross conditions compare the current bar with the
//! previous one.

use std::collections::BTreeMap;

use crate::data::Candle;
use crate::spec::{Condition, IntPredicate, Operand, OperandExpr, PriceField};

/// One bar's computed state: the bar plus every ready indicator's value.
#[derive(Debug, Clone)]
pub struct BarRow {
    /// The bar.
    pub candle: Candle,
    /// Values of indicators that have produced output this bar.
    pub values: BTreeMap<String, f64>,
}

/// Engine state the stateful conditions read.
#[derive(Debug, Clone, Copy)]
pub struct RuleState {
    /// Whether a position is currently open.
    pub in_position: bool,
    /// Bars since the current position was entered (`None` when flat).
    pub bars_since_entry: Option<u32>,
}

fn price(candle: &Candle, field: PriceField) -> f64 {
    match field {
        PriceField::Open => candle.open,
        PriceField::High => candle.high,
        PriceField::Low => candle.low,
        PriceField::Close => candle.close,
        PriceField::Volume => candle.volume,
        PriceField::Hlc3 => candle.hlc3(),
        PriceField::Ohlc4 => candle.ohlc4(),
    }
}

/// Evaluate an operand at bar `idx`, or `None` if it cannot be resolved.
pub fn eval_operand(op: &Operand, history: &[BarRow], idx: usize) -> Option<f64> {
    let row = history.get(idx)?;
    match op {
        Operand::Const(v) => Some(*v),
        // The engine inserts the primary value under the indicator name and each
        // field under "name.field", so a full-name lookup resolves both.
        Operand::Ref(name) => row.values.get(name.as_str()).copied(),
        Operand::Expr(expr) => match expr.as_ref() {
            OperandExpr::Price(field) => Some(price(&row.candle, *field)),
            OperandExpr::Prev((inner, n)) => {
                let i = idx.checked_sub(*n as usize)?;
                eval_operand(inner, history, i)
            }
            OperandExpr::Add((a, b)) => binary(a, b, history, idx, |x, y| x + y),
            OperandExpr::Sub((a, b)) => binary(a, b, history, idx, |x, y| x - y),
            OperandExpr::Mul((a, b)) => binary(a, b, history, idx, |x, y| x * y),
            OperandExpr::Div((a, b)) => binary(a, b, history, idx, |x, y| {
                if y.abs() < f64::EPSILON {
                    f64::NAN
                } else {
                    x / y
                }
            }),
        },
    }
}

fn binary(
    a: &Operand,
    b: &Operand,
    history: &[BarRow],
    idx: usize,
    f: impl Fn(f64, f64) -> f64,
) -> Option<f64> {
    Some(f(
        eval_operand(a, history, idx)?,
        eval_operand(b, history, idx)?,
    ))
}

fn compare(
    a: &Operand,
    b: &Operand,
    history: &[BarRow],
    idx: usize,
    f: impl Fn(f64, f64) -> bool,
) -> bool {
    match (eval_operand(a, history, idx), eval_operand(b, history, idx)) {
        (Some(x), Some(y)) => f(x, y),
        _ => false,
    }
}

fn cross(a: &Operand, b: &Operand, history: &[BarRow], idx: usize, above: bool) -> bool {
    if idx == 0 {
        return false;
    }
    let (Some(an), Some(bn)) = (eval_operand(a, history, idx), eval_operand(b, history, idx))
    else {
        return false;
    };
    let (Some(ap), Some(bp)) = (
        eval_operand(a, history, idx - 1),
        eval_operand(b, history, idx - 1),
    ) else {
        return false;
    };
    if above {
        ap <= bp && an > bn
    } else {
        ap >= bp && an < bn
    }
}

fn int_pred(pred: IntPredicate, n: u32) -> bool {
    match pred {
        IntPredicate::Gt(k) => n > k,
        IntPredicate::Lt(k) => n < k,
        IntPredicate::Ge(k) => n >= k,
        IntPredicate::Le(k) => n <= k,
        IntPredicate::Eq(k) => n == k,
    }
}

/// Evaluate a condition at bar `idx`. Missing operands make a comparison false.
pub fn eval_condition(cond: &Condition, history: &[BarRow], idx: usize, state: RuleState) -> bool {
    match cond {
        Condition::Gt((a, b)) => compare(a, b, history, idx, |x, y| x > y),
        Condition::Lt((a, b)) => compare(a, b, history, idx, |x, y| x < y),
        Condition::Ge((a, b)) => compare(a, b, history, idx, |x, y| x >= y),
        Condition::Le((a, b)) => compare(a, b, history, idx, |x, y| x <= y),
        Condition::Eq((a, b)) => compare(a, b, history, idx, |x, y| (x - y).abs() < f64::EPSILON),
        Condition::Ne((a, b)) => compare(a, b, history, idx, |x, y| (x - y).abs() >= f64::EPSILON),
        Condition::CrossAbove((a, b)) => cross(a, b, history, idx, true),
        Condition::CrossBelow((a, b)) => cross(a, b, history, idx, false),
        Condition::Between((a, lo, hi)) => {
            match (
                eval_operand(a, history, idx),
                eval_operand(lo, history, idx),
                eval_operand(hi, history, idx),
            ) {
                (Some(x), Some(l), Some(h)) => l <= x && x <= h,
                _ => false,
            }
        }
        Condition::Rising((a, n)) => compare_prev(a, *n, history, idx, |now, then| now > then),
        Condition::Falling((a, n)) => compare_prev(a, *n, history, idx, |now, then| now < then),
        Condition::All(cs) => cs.iter().all(|c| eval_condition(c, history, idx, state)),
        Condition::Any(cs) => cs.iter().any(|c| eval_condition(c, history, idx, state)),
        Condition::Not(c) => !eval_condition(c, history, idx, state),
        Condition::InPosition(want) => state.in_position == *want,
        Condition::BarsSinceEntry(pred) => {
            state.bars_since_entry.is_some_and(|n| int_pred(*pred, n))
        }
    }
}

fn compare_prev(
    a: &Operand,
    n: u32,
    history: &[BarRow],
    idx: usize,
    f: impl Fn(f64, f64) -> bool,
) -> bool {
    let Some(prev_idx) = idx.checked_sub(n as usize) else {
        return false;
    };
    match (
        eval_operand(a, history, idx),
        eval_operand(a, history, prev_idx),
    ) {
        (Some(now), Some(then)) => f(now, then),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::StrategySpec;

    fn row(close: f64, vals: &[(&str, f64)]) -> BarRow {
        BarRow {
            candle: Candle {
                time: 0,
                open: close,
                high: close,
                low: close,
                close,
                volume: 0.0,
            },
            values: vals.iter().map(|(k, v)| ((*k).to_string(), *v)).collect(),
        }
    }

    const STATE: RuleState = RuleState {
        in_position: false,
        bars_since_entry: None,
    };

    #[test]
    fn const_and_ref() {
        let h = vec![row(10.0, &[("ema", 5.0)])];
        assert_eq!(eval_operand(&Operand::Const(3.0), &h, 0), Some(3.0));
        assert_eq!(eval_operand(&Operand::Ref("ema".into()), &h, 0), Some(5.0));
        assert_eq!(eval_operand(&Operand::Ref("missing".into()), &h, 0), None);
    }

    #[test]
    fn price_and_prev() {
        let h = vec![row(10.0, &[]), row(20.0, &[])];
        let close = StrategySpec::parse(
            r#"{"symbol":"x","timeframe":"1h","indicators":{},
                "entry":{"gt":[{"price":"close"},{"prev":[{"price":"close"},1]}]},
                "exit":{"in_position":true},"sizing":{"type":"fixed_qty","qty":1}}"#,
        )
        .unwrap();
        assert!(eval_condition(&close.entry, &h, 1, STATE));
        assert!(!eval_condition(&close.entry, &h, 0, STATE)); // no prev at idx 0
    }

    #[test]
    fn cross_above_detects_crossing() {
        // fast crosses slow from below at idx 1.
        let h = vec![
            row(0.0, &[("f", 1.0), ("s", 2.0)]),
            row(0.0, &[("f", 3.0), ("s", 2.0)]),
        ];
        let cond = Condition::CrossAbove((Operand::Ref("f".into()), Operand::Ref("s".into())));
        assert!(!eval_condition(&cond, &h, 0, STATE));
        assert!(eval_condition(&cond, &h, 1, STATE));
    }

    #[test]
    fn all_any_not() {
        let h = vec![row(0.0, &[("a", 5.0)])];
        let gt = Condition::Gt((Operand::Ref("a".into()), Operand::Const(1.0)));
        let lt = Condition::Lt((Operand::Ref("a".into()), Operand::Const(1.0)));
        assert!(eval_condition(
            &Condition::All(vec![gt.clone()]),
            &h,
            0,
            STATE
        ));
        assert!(eval_condition(
            &Condition::Any(vec![lt.clone(), gt.clone()]),
            &h,
            0,
            STATE
        ));
        assert!(eval_condition(&Condition::Not(Box::new(lt)), &h, 0, STATE));
    }
}
