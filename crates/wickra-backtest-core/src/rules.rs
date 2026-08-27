//! Evaluation of the strategy DSL ([`Operand`] / [`Condition`]) against the
//! per-bar history. Operands resolve to a number (or `None` when an indicator
//! is still warming up); conditions resolve to a boolean (false when any
//! operand is missing). Cross conditions compare the current bar with the
//! previous one.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::data::Candle;
use crate::spec::{Condition, IntPredicate, Operand, OperandExpr, PriceField};

/// One bar's computed state: the bar plus every ready indicator's value.
#[derive(Debug, Clone)]
pub struct BarRow {
    /// The bar.
    pub candle: Candle,
    /// Values of indicators that have produced output this bar.
    ///
    /// The keys are shared with the engine rather than owned per row: a long run
    /// otherwise allocated one fresh `String` per value per bar, and cloning an
    /// `Arc<str>` is a refcount bump. Lookup is unchanged -- `Arc<str>` borrows
    /// as `str`, so `values.get("ema_fast")` still works.
    pub values: BTreeMap<Arc<str>, f64>,
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
    lhs: &Operand,
    rhs: &Operand,
    history: &[BarRow],
    idx: usize,
    combine: impl Fn(f64, f64) -> f64,
) -> Option<f64> {
    Some(combine(
        eval_operand(lhs, history, idx)?,
        eval_operand(rhs, history, idx)?,
    ))
}

fn compare(
    lhs: &Operand,
    rhs: &Operand,
    history: &[BarRow],
    idx: usize,
    predicate: impl Fn(f64, f64) -> bool,
) -> bool {
    match (
        eval_operand(lhs, history, idx),
        eval_operand(rhs, history, idx),
    ) {
        (Some(left), Some(right)) => predicate(left, right),
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

/// How many bars *before* the current one an operand can reach.
///
/// The evaluator indexes backwards from the bar being evaluated, so this is what
/// a caller must retain to answer the same questions from a bounded window. It
/// is defined here, beside the code that does the indexing, so the two cannot
/// drift apart: a new backward-looking form has to be given a depth to compile.
#[must_use]
pub fn operand_lookback(op: &Operand) -> usize {
    match op {
        Operand::Ref(_) | Operand::Const(_) => 0,
        Operand::Expr(expr) => match expr.as_ref() {
            OperandExpr::Price(_) => 0,
            // Nested `prev` compounds: prev(prev(x, 2), 3) reaches back five bars.
            OperandExpr::Prev((inner, n)) => *n as usize + operand_lookback(inner),
            OperandExpr::Add((a, b))
            | OperandExpr::Sub((a, b))
            | OperandExpr::Mul((a, b))
            | OperandExpr::Div((a, b)) => operand_lookback(a).max(operand_lookback(b)),
        },
    }
}

/// How many bars *before* the current one a condition can reach.
#[must_use]
pub fn condition_lookback(cond: &Condition) -> usize {
    match cond {
        Condition::Gt((a, b))
        | Condition::Lt((a, b))
        | Condition::Ge((a, b))
        | Condition::Le((a, b))
        | Condition::Eq((a, b))
        | Condition::Ne((a, b)) => operand_lookback(a).max(operand_lookback(b)),
        // A cross compares this bar with the one before it, on both sides.
        Condition::CrossAbove((a, b)) | Condition::CrossBelow((a, b)) => {
            1 + operand_lookback(a).max(operand_lookback(b))
        }
        Condition::Between((a, lo, hi)) => operand_lookback(a)
            .max(operand_lookback(lo))
            .max(operand_lookback(hi)),
        Condition::Rising((a, n)) | Condition::Falling((a, n)) => *n as usize + operand_lookback(a),
        Condition::All(cs) | Condition::Any(cs) => {
            cs.iter().map(condition_lookback).max().unwrap_or(0)
        }
        Condition::Not(c) => condition_lookback(c),
        // Read from RuleState, not from history.
        Condition::InPosition(_) | Condition::BarsSinceEntry(_) => 0,
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
            values: vals.iter().map(|(k, v)| (Arc::from(*k), *v)).collect(),
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

    fn ohlcv_row(open: f64, high: f64, low: f64, close: f64, volume: f64) -> BarRow {
        BarRow {
            candle: Candle {
                time: 0,
                open,
                high,
                low,
                close,
                volume,
            },
            values: BTreeMap::new(),
        }
    }

    fn px(field: PriceField) -> Operand {
        Operand::Expr(Box::new(OperandExpr::Price(field)))
    }

    fn expr(e: OperandExpr) -> Operand {
        Operand::Expr(Box::new(e))
    }

    #[test]
    fn all_price_fields_resolve() {
        let h = vec![ohlcv_row(10.0, 20.0, 5.0, 15.0, 100.0)];
        let at = |f| eval_operand(&px(f), &h, 0).unwrap();
        assert!((at(PriceField::Open) - 10.0).abs() < 1e-9);
        assert!((at(PriceField::High) - 20.0).abs() < 1e-9);
        assert!((at(PriceField::Low) - 5.0).abs() < 1e-9);
        assert!((at(PriceField::Close) - 15.0).abs() < 1e-9);
        assert!((at(PriceField::Volume) - 100.0).abs() < 1e-9);
        assert!((at(PriceField::Hlc3) - (20.0 + 5.0 + 15.0) / 3.0).abs() < 1e-9);
        assert!((at(PriceField::Ohlc4) - (10.0 + 20.0 + 5.0 + 15.0) / 4.0).abs() < 1e-9);
    }

    #[test]
    fn arithmetic_operands() {
        let h = vec![ohlcv_row(0.0, 0.0, 0.0, 0.0, 0.0)];
        let c = |v| Operand::Const(v);
        let ev = |e| eval_operand(&expr(e), &h, 0);
        assert_eq!(
            ev(OperandExpr::Add((Box::new(c(2.0)), Box::new(c(3.0))))),
            Some(5.0)
        );
        assert_eq!(
            ev(OperandExpr::Sub((Box::new(c(2.0)), Box::new(c(3.0))))),
            Some(-1.0)
        );
        assert_eq!(
            ev(OperandExpr::Mul((Box::new(c(2.0)), Box::new(c(3.0))))),
            Some(6.0)
        );
        assert_eq!(
            ev(OperandExpr::Div((Box::new(c(6.0)), Box::new(c(3.0))))),
            Some(2.0)
        );
        // Division by zero yields NaN, not a panic.
        let nan = ev(OperandExpr::Div((Box::new(c(1.0)), Box::new(c(0.0))))).unwrap();
        assert!(nan.is_nan());
    }

    #[test]
    fn arithmetic_with_missing_operand_is_none() {
        let h = vec![ohlcv_row(0.0, 0.0, 0.0, 0.0, 0.0)];
        let miss = Box::new(Operand::Ref("nope".into()));
        let got = eval_operand(
            &expr(OperandExpr::Add((miss, Box::new(Operand::Const(1.0))))),
            &h,
            0,
        );
        assert_eq!(got, None);
    }

    #[test]
    fn prev_underflow_is_none() {
        let h = vec![ohlcv_row(0.0, 0.0, 0.0, 0.0, 0.0)];
        let got = eval_operand(
            &expr(OperandExpr::Prev((Box::new(px(PriceField::Close)), 3))),
            &h,
            0,
        );
        assert_eq!(got, None);
    }

    #[test]
    fn comparisons_ge_le_eq_ne() {
        let h = vec![row(0.0, &[("a", 2.0)])];
        let a = Operand::Ref("a".into());
        let two = Operand::Const(2.0);
        let three = Operand::Const(3.0);
        assert!(eval_condition(
            &Condition::Ge((a.clone(), two.clone())),
            &h,
            0,
            STATE
        ));
        assert!(eval_condition(
            &Condition::Le((a.clone(), two.clone())),
            &h,
            0,
            STATE
        ));
        assert!(eval_condition(
            &Condition::Eq((a.clone(), two.clone())),
            &h,
            0,
            STATE
        ));
        assert!(eval_condition(
            &Condition::Ne((a.clone(), three)),
            &h,
            0,
            STATE
        ));
        // A missing operand makes any comparison false.
        let miss = Operand::Ref("x".into());
        assert!(!eval_condition(&Condition::Gt((miss, two)), &h, 0, STATE));
    }

    #[test]
    fn cross_below_detects_crossing() {
        let h = vec![
            row(0.0, &[("f", 3.0), ("s", 2.0)]),
            row(0.0, &[("f", 1.0), ("s", 2.0)]),
        ];
        let cond = Condition::CrossBelow((Operand::Ref("f".into()), Operand::Ref("s".into())));
        assert!(!eval_condition(&cond, &h, 0, STATE));
        assert!(eval_condition(&cond, &h, 1, STATE));
    }

    #[test]
    fn between_inside_and_outside() {
        let h = vec![row(0.0, &[("a", 5.0)])];
        let a = Operand::Ref("a".into());
        let inside = Condition::Between((a.clone(), Operand::Const(1.0), Operand::Const(10.0)));
        let outside = Condition::Between((a.clone(), Operand::Const(6.0), Operand::Const(10.0)));
        assert!(eval_condition(&inside, &h, 0, STATE));
        assert!(!eval_condition(&outside, &h, 0, STATE));
        // Missing operand -> false.
        let miss = Condition::Between((
            Operand::Ref("x".into()),
            Operand::Const(1.0),
            Operand::Const(10.0),
        ));
        assert!(!eval_condition(&miss, &h, 0, STATE));
    }

    #[test]
    fn rising_and_falling() {
        let up = vec![row(0.0, &[("a", 1.0)]), row(0.0, &[("a", 2.0)])];
        let down = vec![row(0.0, &[("a", 2.0)]), row(0.0, &[("a", 1.0)])];
        let a = Operand::Ref("a".into());
        assert!(eval_condition(
            &Condition::Rising((a.clone(), 1)),
            &up,
            1,
            STATE
        ));
        assert!(eval_condition(
            &Condition::Falling((a.clone(), 1)),
            &down,
            1,
            STATE
        ));
        // Not enough history -> false.
        assert!(!eval_condition(&Condition::Rising((a, 5)), &up, 1, STATE));
    }

    #[test]
    fn in_position_and_bars_since_entry() {
        let h = vec![row(0.0, &[])];
        let open_state = RuleState {
            in_position: true,
            bars_since_entry: Some(5),
        };
        assert!(eval_condition(
            &Condition::InPosition(true),
            &h,
            0,
            open_state
        ));
        assert!(!eval_condition(&Condition::InPosition(true), &h, 0, STATE));

        // Each integer predicate against bars_since_entry = 5.
        let cases = [
            (IntPredicate::Gt(4), true),
            (IntPredicate::Lt(6), true),
            (IntPredicate::Ge(5), true),
            (IntPredicate::Le(5), true),
            (IntPredicate::Eq(5), true),
            (IntPredicate::Eq(4), false),
        ];
        for (pred, want) in cases {
            assert_eq!(
                eval_condition(&Condition::BarsSinceEntry(pred), &h, 0, open_state),
                want
            );
        }
        // Flat: bars_since_entry is None -> always false.
        assert!(!eval_condition(
            &Condition::BarsSinceEntry(IntPredicate::Ge(0)),
            &h,
            0,
            STATE
        ));
    }
}
