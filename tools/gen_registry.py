#!/usr/bin/env python3
"""Generate crates/wickra-backtest-core/src/registry.rs.

Single source of truth: the wickra repo's wasm binding macros
(bindings/wasm/src/lib.rs), which already compile against wickra-core and so
carry the exact Rust constructor signatures:

    wasm_scalar_indicator!(WasmX, "ALIAS", wc::Type, p1: usize, p2: f64)
        -> Input = f64 single-output indicators (fed the bar close)
    wasm_candle_pattern!(WasmX, wc::Type, Js)
        -> Input = Candle, param-less new(), +-1 / scalar output

Default constructor parameters for the build-all test come from the wickra
golden manifest (testdata/golden/scalar_manifest.json), joined by canonical
name.

A small set of candle-input scalar indicators and the multi-output indicators
are kept hand-written (they have no uniform wasm macro); they are emitted
verbatim from the templates below.

Usage (with a sibling wickra checkout):
    python tools/gen_registry.py --wickra ../wickra \
        --out crates/wickra-backtest-core/src/registry.rs
"""
from __future__ import annotations

import argparse
import json
import re
from pathlib import Path

# Candle-input scalar indicators kept hand-written (no uniform wasm macro).
HAND_CANDLE_SCALAR = {
    "Atr": ("p(0)?", [14.0]),
    "Cci": ("p(0)?", [20.0]),
    "WilliamsR": ("p(0)?", [14.0]),
    "Mfi": ("p(0)?", [14.0]),
}
# Candle-input scalar with a param-less, non-Result constructor.
HAND_CANDLE_SCALAR_NOARG = {
    "Vwap": [],
    "Obv": [],
}

ARG_READER = {
    "usize": "p({i})?",
    "f64": "float_param(params, {i}, kind)?",
    "u32": "u32_param(params, {i}, kind)?",
}


def parse_scalar(src: str):
    """Return [(type, [(argname, argtype), ...]), ...] for wasm_scalar_indicator!."""
    out = []
    for body in re.findall(r"wasm_scalar_indicator!\s*\((.*?)\)\s*;", src, re.S):
        parts = [p.strip() for p in body.split(",")]
        ty = None
        args = []
        for p in parts:
            if p.startswith("wc::"):
                ty = p[4:]
            elif ":" in p and not p.startswith('"'):
                name, t = p.split(":", 1)
                args.append((name.strip(), t.strip()))
        if ty:
            out.append((ty, args))
    return out


def parse_candle_patterns(src: str):
    """Return [type, ...] for wasm_candle_pattern! (the 2nd, wc:: argument)."""
    out = []
    for body in re.findall(r"wasm_candle_pattern!\s*\((.*?)\)\s*;", src, re.S):
        for p in (x.strip() for x in body.split(",")):
            if p.startswith("wc::"):
                out.append(p[4:])
                break
    return out


def reader(argtype: str, i: int) -> str:
    tmpl = ARG_READER.get(argtype)
    if tmpl is None:
        raise SystemExit(f"unsupported ctor arg type: {argtype}")
    return tmpl.format(i=i)


HEAD = r'''//! Indicator registry: constructs `wickra-core` indicators by name and wraps
//! them behind a uniform, object-safe [`EvalIndicator`] the engine can drive
//! from a [`Candle`].
//!
//! GENERATED FILE — do not edit by hand. Regenerate with:
//!
//! ```text
//! python tools/gen_registry.py --wickra ../wickra --out crates/wickra-backtest-core/src/registry.rs
//! ```
//!
//! Source of truth: the wickra wasm binding macros (exact Rust constructor
//! signatures) joined with the golden manifest (default parameters). Scalar
//! (`Input = f64`) and candlestick-pattern (`Input = Candle`, param-less)
//! indicators are generated; a few candle-input scalar indicators and the
//! multi-output indicators are kept hand-written.

use wickra_core::{self as wc, Candle as CoreCandle, Indicator};

use crate::data::Candle;
use crate::error::{BacktestError, Result};

/// A uniform, object-safe indicator the engine drives one bar at a time.
pub trait EvalIndicator: Send {
    /// Feed one bar; returns the primary value, or `None` while warming up.
    fn update(&mut self, candle: &Candle) -> Option<f64>;
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
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        self.0.update(candle.close)
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
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        candle.to_core().ok().and_then(|c| self.0.update(c))
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        Vec::new()
    }
    fn warmup(&self) -> usize {
        self.0.warmup_period()
    }
}

/// MACD (`{macd, signal, histogram}`); primary value is the MACD line.
struct MacdWrap {
    inner: wc::MacdIndicator,
    last: Vec<(&'static str, f64)>,
}

impl EvalIndicator for MacdWrap {
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        let out = self.inner.update(candle.close)?;
        self.last = vec![
            ("macd", out.macd),
            ("signal", out.signal),
            ("histogram", out.histogram),
        ];
        Some(out.macd)
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        self.last.clone()
    }
    fn warmup(&self) -> usize {
        self.inner.warmup_period()
    }
}

/// Bollinger Bands (`{upper, middle, lower}`); primary value is the middle band.
struct BollingerWrap {
    inner: wc::BollingerBands,
    last: Vec<(&'static str, f64)>,
}

impl EvalIndicator for BollingerWrap {
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        let out = self.inner.update(candle.close)?;
        self.last = vec![
            ("upper", out.upper),
            ("middle", out.middle),
            ("lower", out.lower),
        ];
        Some(out.middle)
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        self.last.clone()
    }
    fn warmup(&self) -> usize {
        self.inner.warmup_period()
    }
}

/// Stochastic oscillator (`{k, d}`); primary value is `%K`.
struct StochasticWrap {
    inner: wc::Stochastic,
    last: Vec<(&'static str, f64)>,
}

impl EvalIndicator for StochasticWrap {
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        let out = candle.to_core().ok().and_then(|c| self.inner.update(c))?;
        self.last = vec![("k", out.k), ("d", out.d)];
        Some(out.k)
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        self.last.clone()
    }
    fn warmup(&self) -> usize {
        self.inner.warmup_period()
    }
}

/// ADX (`{adx, plus_di, minus_di}`); primary value is the ADX line.
struct AdxWrap {
    inner: wc::Adx,
    last: Vec<(&'static str, f64)>,
}

impl EvalIndicator for AdxWrap {
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        let out = candle.to_core().ok().and_then(|c| self.inner.update(c))?;
        self.last = vec![
            ("adx", out.adx),
            ("plus_di", out.plus_di),
            ("minus_di", out.minus_di),
        ];
        Some(out.adx)
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        self.last.clone()
    }
    fn warmup(&self) -> usize {
        self.inner.warmup_period()
    }
}

/// Aroon (`{up, down}`); primary value is Aroon-Up.
struct AroonWrap {
    inner: wc::Aroon,
    last: Vec<(&'static str, f64)>,
}

impl EvalIndicator for AroonWrap {
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        let out = candle.to_core().ok().and_then(|c| self.inner.update(c))?;
        self.last = vec![("up", out.up), ("down", out.down)];
        Some(out.up)
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        self.last.clone()
    }
    fn warmup(&self) -> usize {
        self.inner.warmup_period()
    }
}

/// Keltner Channels (`{upper, middle, lower}`); primary value is the middle line.
struct KeltnerWrap {
    inner: wc::Keltner,
    last: Vec<(&'static str, f64)>,
}

impl EvalIndicator for KeltnerWrap {
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        let out = candle.to_core().ok().and_then(|c| self.inner.update(c))?;
        self.last = vec![
            ("upper", out.upper),
            ("middle", out.middle),
            ("lower", out.lower),
        ];
        Some(out.middle)
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        self.last.clone()
    }
    fn warmup(&self) -> usize {
        self.inner.warmup_period()
    }
}

/// Donchian Channels (`{upper, middle, lower}`); primary value is the middle line.
struct DonchianWrap {
    inner: wc::Donchian,
    last: Vec<(&'static str, f64)>,
}

impl EvalIndicator for DonchianWrap {
    fn update(&mut self, candle: &Candle) -> Option<f64> {
        let out = candle.to_core().ok().and_then(|c| self.inner.update(c))?;
        self.last = vec![
            ("upper", out.upper),
            ("middle", out.middle),
            ("lower", out.lower),
        ];
        Some(out.middle)
    }
    fn fields(&self) -> Vec<(&'static str, f64)> {
        self.last.clone()
    }
    fn warmup(&self) -> usize {
        self.inner.warmup_period()
    }
}

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
    let _ = &p;
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

    #[test]
    fn builds_all_known_indicators() {
        for (kind, params) in ALL_SPECS {
            assert!(build(kind, params).is_ok(), "{kind} should build");
        }
    }

    #[test]
    fn registry_has_full_catalog() {
        // Generated scalar + candlestick families plus the hand-written set.
        assert!(ALL_SPECS.len() >= 200, "catalog too small: {}", ALL_SPECS.len());
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
        assert!(build("Macd", &[12.0, 26.0]).is_err()); // missing signal
        assert!(build("Bollinger", &[20.0]).is_err()); // missing multiplier
    }

    #[test]
    fn macd_exposes_fields() {
        let mut macd = build("Macd", &[2.0, 3.0, 2.0]).unwrap();
        let mut last_fields = Vec::new();
        for px in [10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 17.0] {
            if macd.update(&candle(px, px, px)).is_some() {
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
        sma.update(&candle(10.0, 10.0, 10.0));
        sma.update(&candle(20.0, 20.0, 20.0));
        assert!(sma.fields().is_empty());
    }
}
'''


def fmt_params(vals):
    return ", ".join(f"{float(v)}" for v in vals)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--wickra", required=True, help="path to a wickra checkout")
    ap.add_argument("--out", required=True, help="output registry.rs path")
    args = ap.parse_args()

    wroot = Path(args.wickra)
    wasm = (wroot / "bindings/wasm/src/lib.rs").read_text("utf-8")
    manifest = json.loads(
        (wroot / "testdata/golden/scalar_manifest.json").read_text("utf-8")
    )
    default_params = {e["canonical"]: e["params"] for e in manifest}

    scalars = parse_scalar(wasm)
    candles = parse_candle_patterns(wasm)

    # Multi-output hand arms (kind -> (ctor expr, default params)).
    multi = {
        "Macd": ("MacdWrap { inner: map_new(kind, wc::MacdIndicator::new(p(0)?, p(1)?, p(2)?))?, last: Vec::new() }", [12.0, 26.0, 9.0]),
        "Bollinger": ("BollingerWrap { inner: map_new(kind, wc::BollingerBands::new(p(0)?, float_param(params, 1, kind)?))?, last: Vec::new() }", [20.0, 2.0]),
        "Stochastic": ("StochasticWrap { inner: map_new(kind, wc::Stochastic::new(p(0)?, p(1)?))?, last: Vec::new() }", [14.0, 3.0]),
        "Adx": ("AdxWrap { inner: map_new(kind, wc::Adx::new(p(0)?))?, last: Vec::new() }", [14.0]),
        "Aroon": ("AroonWrap { inner: map_new(kind, wc::Aroon::new(p(0)?))?, last: Vec::new() }", [14.0]),
        "Keltner": ("KeltnerWrap { inner: map_new(kind, wc::Keltner::new(p(0)?, p(1)?, float_param(params, 2, kind)?))?, last: Vec::new() }", [20.0, 10.0, 2.0]),
        "Donchian": ("DonchianWrap { inner: map_new(kind, wc::Donchian::new(p(0)?))?, last: Vec::new() }", [20.0]),
    }

    arms = []
    specs = []  # (kind, [params]) for the build-all test
    seen = set()

    def add(kind, arm, test_params):
        if kind in seen:
            return
        seen.add(kind)
        arms.append(arm)
        specs.append((kind, test_params))

    # Generated scalar (f64-input) indicators.
    arms.append("        // --- generated scalar (Input = f64), fed the close ---")
    for ty, cargs in scalars:
        readers = ", ".join(reader(t, i) for i, (_, t) in enumerate(cargs))
        ctor = f"wc::{ty}::new({readers})" if readers else f"wc::{ty}::new()"
        arm = f'        "{ty}" => Ok(Box::new(ScalarClose(map_new(kind, {ctor})?))),'
        tp = default_params.get(ty)
        if tp is None:
            tp = [14.0 if t == "usize" else (2 if t == "u32" else 2.0) for _, t in cargs]
        add(ty, arm, tp)

    # Generated candlestick patterns (Input = Candle, param-less new()).
    arms.append("        // --- generated candlestick patterns (Input = Candle) ---")
    for ty in candles:
        arm = f'        "{ty}" => Ok(Box::new(CandleIn(wc::{ty}::new()))),'
        add(ty, arm, [])

    # Hand-written candle-input scalar indicators.
    arms.append("        // --- hand-written candle-input scalar indicators ---")
    for ty, (rd, tp) in HAND_CANDLE_SCALAR.items():
        arm = f'        "{ty}" => Ok(Box::new(CandleIn(map_new(kind, wc::{ty}::new({rd}))?))),'
        add(ty, arm, tp)
    for ty, tp in HAND_CANDLE_SCALAR_NOARG.items():
        arm = f'        "{ty}" => Ok(Box::new(CandleIn(wc::{ty}::new()))),'
        add(ty, arm, tp)

    # Hand-written multi-output indicators.
    arms.append("        // --- hand-written multi-output indicators ---")
    for kind, (expr, tp) in multi.items():
        arm = f'        "{kind}" => Ok(Box::new({expr})),'
        add(kind, arm, tp)

    arms.append("        other => Err(BacktestError::UnknownIndicator(other.to_string())),")

    # Emit ALL_SPECS const for the build-all test.
    spec_lines = "\n".join(
        f'    ("{k}", &[{fmt_params(pp)}]),' for k, pp in specs
    )
    n_real = len(seen)
    specs_const = (
        f"\n/// Every registered indicator with valid default parameters "
        f"({n_real} indicators).\n"
        f"#[cfg(test)]\n"
        f"const ALL_SPECS: &[(&str, &[f64])] = &[\n{spec_lines}\n];\n"
    )

    body = HEAD + "\n".join(arms) + "\n    }\n}\n" + specs_const + FOOT_TESTS
    Path(args.out).write_text(body, encoding="utf-8")
    print(f"registry: {n_real} indicators "
          f"({len(scalars)} scalar + {len(candles)} candlestick + "
          f"{len(HAND_CANDLE_SCALAR) + len(HAND_CANDLE_SCALAR_NOARG)} hand candle-scalar + "
          f"{len(multi)} multi) -> {args.out}")


if __name__ == "__main__":
    main()
