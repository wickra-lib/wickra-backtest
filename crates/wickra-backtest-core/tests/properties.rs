//! Property tests: the parser and engine must never panic — they always return
//! a `Result`, whatever the input. This gives fuzz-like coverage on the stable
//! toolchain (the libfuzzer targets only build on nightly), guarding the
//! trust-critical promise that no strategy input can crash the backtester.

use proptest::prelude::*;
use wickra_backtest_core::{run_json, run_with_capital, Candle, StrategySpec};

/// A single valid candle: `low <= open, close <= high`, all finite and positive.
fn valid_candle() -> impl Strategy<Value = Candle> {
    (
        1.0f64..1_000_000.0,
        0.0f64..10_000.0,
        0.0f64..10_000.0,
        0.0f64..1.0,
        0.0f64..1.0,
    )
        .prop_map(|(base, up, down, open_frac, close_frac)| {
            let low = (base - down).max(0.0001);
            let high = (base + up).max(low + 0.0001);
            Candle {
                time: 0,
                open: low + open_frac * (high - low),
                high,
                low,
                close: low + close_frac * (high - low),
                volume: 0.0,
            }
        })
}

fn price_threshold_spec() -> StrategySpec {
    StrategySpec::parse(
        r#"{"symbol":"x","timeframe":"1h","indicators":{},
            "entry":{"gt":[{"price":"close"},500]},
            "exit":{"lt":[{"price":"close"},500]},
            "sizing":{"type":"fixed_fraction","fraction":0.5}}"#,
    )
    .unwrap()
}

proptest! {
    /// Parsing arbitrary text never panics — it returns `Ok` or `Err`.
    #[test]
    fn parse_never_panics(s in ".*") {
        let _ = StrategySpec::parse(&s);
    }

    /// `run_json` on arbitrary text never panics.
    #[test]
    fn run_json_never_panics(s in ".*") {
        let _ = run_json(&s);
    }

    /// The engine never panics on any sequence of valid candles, and every
    /// reported metric is finite.
    #[test]
    fn engine_never_panics_on_valid_candles(
        mut candles in proptest::collection::vec(valid_candle(), 1..60)
    ) {
        // Give the candles strictly increasing timestamps.
        for (i, c) in candles.iter_mut().enumerate() {
            c.time = i64::try_from(i).unwrap();
        }
        let spec = price_threshold_spec();
        let report = run_with_capital(&spec, &candles, 10_000.0).expect("run");
        let m = &report.metrics;
        prop_assert!(m.pnl.is_finite());
        prop_assert!(m.return_pct.is_finite());
        prop_assert!(m.sharpe.is_finite());
        prop_assert!(m.sortino.is_finite());
        prop_assert!(m.max_drawdown.is_finite());
        // Calmar may be infinite by definition (zero drawdown); just not NaN.
        prop_assert!(!m.calmar.is_nan());
        prop_assert_eq!(report.equity.len(), candles.len());
    }
}
