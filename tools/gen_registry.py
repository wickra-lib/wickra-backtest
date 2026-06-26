#!/usr/bin/env python3
"""Generate crates/wickra-backtest-core/src/registry.rs.

Single source of truth: the wickra-core indicator sources themselves
(crates/wickra-core/src/indicators/*.rs). For every type that implements the
`Indicator` trait we read, directly from the source:

  - the associated `type Input` (f64 / Candle / ... ) and `type Output`
  - the `pub [const] fn new(...) -> Result<Self> | Self` constructor signature
  - for multi-output indicators, the `f64` field names of the Output struct

Every indicator whose input is a single instrument's price (`f64`, fed the
close) or candle (`Candle`) and whose output is a scalar `f64` or a struct of
`f64` fields is emitted, plus pairwise `(f64, f64)` scalar-output indicators
fed `(close, reference_close)` from the reference series. Cross-section,
derivatives, trade, order-book and quote inputs (and pairwise multi-output) are
out of scope here and are skipped (and reported).

Default constructor parameters for the build-all test come from the wickra
golden manifests, joined by canonical name.

Usage (with a sibling wickra checkout):
    python tools/gen_registry.py --wickra ../wickra \
        --out crates/wickra-backtest-core/src/registry.rs
"""
from __future__ import annotations

import argparse
import glob
import json
import re
from collections import Counter
from pathlib import Path

# Friendly aliases kept for ergonomics / backward compatibility. Each maps a
# short name to the canonical wickra-core type name.
ALIASES = {
    "Macd": "MacdIndicator",
    "Bollinger": "BollingerBands",
}

# usize/f64/u32/i32 are read with the helpers below; anything else is skipped.
ARG_READER = {
    "usize": "p({i})?",
    "f64": "float_param(params, {i}, kind)?",
    "u32": "u32_param(params, {i}, kind)?",
    "i32": "i32_param(params, {i}, kind)?",
}


def assoc_types(text: str, ty: str):
    m = re.search(r"impl\s+Indicator\s+for\s+" + re.escape(ty) + r"\b", text)
    if not m:
        return None, None
    seg = text[m.end(): m.end() + 2000]
    mi = re.search(r"type\s+Input\s*=\s*([^;]+);", seg)
    mo = re.search(r"type\s+Output\s*=\s*([^;]+);", seg)
    inp = re.sub(r"\s+", "", mi.group(1)) if mi else None
    out = mo.group(1).strip() if mo else None
    return inp, out


def find_new(text: str, ty: str):
    """Return (arg_types, returns_result) for `pub [const] fn new`, or None."""
    for m in re.finditer(r"impl\s+" + re.escape(ty) + r"\s*\{", text):
        seg = text[m.end(): m.end() + 3000]
        mn = re.search(
            r"pub\s+(?:const\s+)?fn\s+new\s*\(([^)]*)\)\s*->\s*(Result<Self>|Self)\s*\{",
            seg,
            re.S,
        )
        if mn:
            argstr = mn.group(1).strip()
            argtypes = [
                p.split(":", 1)[1].strip() for p in argstr.split(",") if ":" in p
            ]
            return argtypes, mn.group(2).strip() == "Result<Self>"
    return None


def out_fields(bigtext: str, out: str):
    m = re.search(r"pub\s+struct\s+" + re.escape(out) + r"\s*\{(.*?)\n\}", bigtext, re.S)
    if not m:
        return None
    return re.findall(r"pub\s+(\w+)\s*:\s*f64\b", m.group(1))


def readers(argtypes):
    return ", ".join(ARG_READER[t].format(i=i) for i, t in enumerate(argtypes))


def fmt_params(vals):
    return ", ".join(f"{float(v)}" for v in vals)


HEAD = r'''//! Indicator registry: constructs `wickra-core` indicators by name and wraps
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
    self as wc, Candle as CoreCandle, DerivativesTick as CoreDerivativesTick,
    OrderBook as CoreOrderBook, Indicator,
};

use crate::data::Candle;
use crate::error::{BacktestError, Result};

/// Everything an indicator may consume on one bar. Single-instrument indicators
/// use `candle`; pairwise indicators also use `reference`; derivatives and
/// order-book indicators use `deriv` / `orderbook`. Feeds that are absent are
/// `None`.
pub struct BarInput<'a> {
    /// The current bar.
    pub candle: &'a Candle,
    /// The reference series' close (for pairwise indicators).
    pub reference: Option<f64>,
    /// The derivatives tick for this bar (for derivatives indicators).
    pub deriv: Option<CoreDerivativesTick>,
    /// The order-book snapshot for this bar (for order-book indicators).
    pub orderbook: Option<&'a CoreOrderBook>,
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

'''

HELPERS = r'''
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
    if v.fract().abs() > f64::EPSILON
        || v > f64::from(i32::MAX)
        || v < f64::from(i32::MIN)
    {
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
'''

FOOT_TESTS = r'''
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
'''


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--wickra", required=True)
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    wroot = Path(args.wickra)
    ind_dir = wroot / "crates/wickra-core/src/indicators"
    files = {
        Path(f).name: Path(f).read_text("utf-8")
        for f in glob.glob(str(ind_dir / "*.rs"))
        if Path(f).name != "mod.rs"
    }
    bigtext = "\n".join(files.values())

    default_params = {}
    for man in ("scalar_manifest.json", "multi_manifest.json"):
        for e in json.loads((wroot / "testdata/golden" / man).read_text("utf-8")):
            default_params[e["canonical"]] = e["params"]

    scalars = []  # (ty, input, args, is_result)
    multis = []   # (ty, input, args, is_result, fields)
    pairs = []    # (ty, args, is_result) for Input = (f64, f64), Output = f64
    pair_multis = []  # (ty, args, is_result, fields) for pairwise struct output
    deriv_scalars = []  # (ty, args, is_result) for Input = DerivativesTick, f64 out
    deriv_multis = []   # (ty, args, is_result, fields) for derivatives struct out
    ob_scalars = []     # (ty, args, is_result) for Input = OrderBook, f64 out
    skip = Counter()

    for text in files.values():
        for m in re.finditer(r"\nimpl\s+Indicator\s+for\s+(\w+)\b", text):
            ty = m.group(1)
            inp, out = assoc_types(text, ty)
            if inp not in ("f64", "Candle", "(f64,f64)", "DerivativesTick", "OrderBook"):
                if inp:
                    skip[f"input {inp}"] += 1
                continue
            nr = find_new(text, ty)
            if nr is None:
                skip["no new()"] += 1
                continue
            argtypes, is_result = nr
            bad = [a for a in argtypes if a not in ARG_READER]
            if bad:
                skip[f"arg {','.join(bad)}"] += 1
                continue
            if inp == "(f64,f64)":
                if out == "f64":
                    pairs.append((ty, argtypes, is_result))
                else:
                    fields = out_fields(bigtext, out)
                    if fields:
                        pair_multis.append((ty, argtypes, is_result, fields))
                    else:
                        skip[f"pairwise no f64 fields ({out})"] += 1
                continue
            if inp == "DerivativesTick":
                if out == "f64":
                    deriv_scalars.append((ty, argtypes, is_result))
                else:
                    fields = out_fields(bigtext, out)
                    if fields:
                        deriv_multis.append((ty, argtypes, is_result, fields))
                    else:
                        skip[f"derivatives no f64 fields ({out})"] += 1
                continue
            if inp == "OrderBook":
                if out == "f64":
                    ob_scalars.append((ty, argtypes, is_result))
                else:
                    skip[f"order-book non-scalar ({out})"] += 1
                continue
            if out == "f64":
                scalars.append((ty, inp, argtypes, is_result))
            else:
                fields = out_fields(bigtext, out)
                if not fields:
                    skip[f"no f64 fields ({out})"] += 1
                    continue
                multis.append((ty, inp, argtypes, is_result, fields))

    scalars.sort()
    multis.sort()
    pairs.sort()
    pair_multis.sort()
    deriv_scalars.sort()
    deriv_multis.sort()
    ob_scalars.sort()

    # Emit multi-wrapper macro invocations.
    wraps = []
    for ty, inp, _args, _res, fields in multis:
        mac = "multi_close" if inp == "f64" else "multi_candle"
        flist = ", ".join(fields)
        wraps.append(f"{mac}!({ty}Wrap, {ty}, {fields[0]}, [{flist}]);")
    for ty, _args, _res, fields in pair_multis:
        flist = ", ".join(fields)
        wraps.append(f"multi_pair!({ty}Wrap, {ty}, {fields[0]}, [{flist}]);")
    for ty, _args, _res, fields in deriv_multis:
        flist = ", ".join(fields)
        wraps.append(f"multi_deriv!({ty}Wrap, {ty}, {fields[0]}, [{flist}]);")

    arms = []
    specs = []
    seen = set()

    def default_for(ty, argtypes):
        tp = default_params.get(ty)
        if tp is not None:
            return tp
        synth = {"usize": 14.0, "f64": 2.0, "u32": 2.0, "i32": 1.0}
        return [synth[a] for a in argtypes]

    def ctor_expr(ty, argtypes, is_result):
        rd = readers(argtypes)
        call = f"wc::{ty}::new({rd})"
        return f"map_new(kind, {call})?" if is_result else call

    arms.append("        // --- scalar single-output (Input = f64), fed the close ---")
    for ty, inp, argtypes, is_result in scalars:
        if inp != "f64":
            continue
        arm = f'        "{ty}" => Ok(Box::new(ScalarClose({ctor_expr(ty, argtypes, is_result)}))),'
        arms.append(arm)
        seen.add(ty)
        specs.append((ty, default_for(ty, argtypes)))

    arms.append("        // --- scalar single-output (Input = Candle) ---")
    for ty, inp, argtypes, is_result in scalars:
        if inp != "Candle":
            continue
        arm = f'        "{ty}" => Ok(Box::new(CandleIn({ctor_expr(ty, argtypes, is_result)}))),'
        arms.append(arm)
        seen.add(ty)
        specs.append((ty, default_for(ty, argtypes)))

    arms.append("        // --- multi-output indicators (named fields) ---")
    for ty, inp, argtypes, is_result, _fields in multis:
        arm = f'        "{ty}" => Ok(Box::new({ty}Wrap::wrap({ctor_expr(ty, argtypes, is_result)}))),'
        arms.append(arm)
        seen.add(ty)
        specs.append((ty, default_for(ty, argtypes)))

    arms.append("        // --- pairwise indicators, fed (close, reference_close) ---")
    for ty, argtypes, is_result in pairs:
        arm = f'        "{ty}" => Ok(Box::new(PairClose({ctor_expr(ty, argtypes, is_result)}))),'
        arms.append(arm)
        seen.add(ty)
        specs.append((ty, default_for(ty, argtypes)))

    arms.append("        // --- pairwise multi-output indicators ---")
    for ty, argtypes, is_result, _fields in pair_multis:
        arm = f'        "{ty}" => Ok(Box::new({ty}Wrap::wrap({ctor_expr(ty, argtypes, is_result)}))),'
        arms.append(arm)
        seen.add(ty)
        specs.append((ty, default_for(ty, argtypes)))

    arms.append("        // --- derivatives indicators, fed the bar's DerivativesTick ---")
    for ty, argtypes, is_result in deriv_scalars:
        arm = f'        "{ty}" => Ok(Box::new(DerivativesIn({ctor_expr(ty, argtypes, is_result)}))),'
        arms.append(arm)
        seen.add(ty)
        specs.append((ty, default_for(ty, argtypes)))
    for ty, argtypes, is_result, _fields in deriv_multis:
        arm = f'        "{ty}" => Ok(Box::new({ty}Wrap::wrap({ctor_expr(ty, argtypes, is_result)}))),'
        arms.append(arm)
        seen.add(ty)
        specs.append((ty, default_for(ty, argtypes)))

    arms.append("        // --- order-book indicators, fed the bar's OrderBook ---")
    for ty, argtypes, is_result in ob_scalars:
        arm = f'        "{ty}" => Ok(Box::new(OrderBookIn({ctor_expr(ty, argtypes, is_result)}))),'
        arms.append(arm)
        seen.add(ty)
        specs.append((ty, default_for(ty, argtypes)))

    arms.append("        // --- friendly aliases ---")
    for alias, canon in ALIASES.items():
        if canon in seen:
            arms.append(f'        "{alias}" => build("{canon}", params),')

    arms.append("        other => Err(BacktestError::UnknownIndicator(other.to_string())),")

    spec_lines = "\n".join(f'    ("{k}", &[{fmt_params(pp)}]),' for k, pp in specs)
    specs_const = (
        f"\n/// Every registered indicator with valid default parameters "
        f"({len(seen)} indicators).\n"
        f"#[cfg(test)]\n"
        f"const ALL_SPECS: &[(&str, &[f64])] = &[\n{spec_lines}\n];\n"
    )

    body = (
        HEAD
        + "\n".join(wraps)
        + "\n"
        + HELPERS
        + "\n".join(arms)
        + "\n    }\n}\n"
        + specs_const
        + FOOT_TESTS
    )
    Path(args.out).write_text(body, encoding="utf-8")

    print(f"registry: {len(seen)} indicators ({len(scalars)} scalar + "
          f"{len(multis)} multi + {len(pairs)} pairwise + "
          f"{len(pair_multis)} pairwise-multi + "
          f"{len(deriv_scalars) + len(deriv_multis)} derivatives + "
          f"{len(ob_scalars)} order-book) + {len(ALIASES)} aliases -> {args.out}")
    print("skipped (structurally out of scope):")
    for k, v in skip.most_common():
        print(f"  {v:3} {k}")


if __name__ == "__main__":
    main()
