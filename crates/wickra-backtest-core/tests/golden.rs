//! Golden parity anchor: the Rust engine turns each shared case
//! (`golden/cases/*.json`) into the canonical expected report
//! (`golden/expected/*.json`). Every language binding asserts its own output
//! against the same expected reports, so cross-language equality is pinned.
//!
//! Regenerate the expected reports after an intentional engine change:
//!
//! ```text
//! WICKRA_BLESS=1 cargo test -p wickra-backtest-core --test golden
//! ```

use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use wickra_backtest_core::{run_with_capital, Candle, StrategySpec};

#[derive(Deserialize)]
struct Case {
    name: String,
    capital: f64,
    spec: serde_json::Value,
    open: Vec<f64>,
    high: Vec<f64>,
    low: Vec<f64>,
    close: Vec<f64>,
    volume: Vec<f64>,
    time: Vec<i64>,
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../golden")
}

fn run_case(case: &Case) -> String {
    let spec = StrategySpec::parse(&case.spec.to_string()).expect("spec parses");
    let candles: Vec<Candle> = (0..case.close.len())
        .map(|i| Candle {
            time: case.time[i],
            open: case.open[i],
            high: case.high[i],
            low: case.low[i],
            close: case.close[i],
            volume: case.volume[i],
        })
        .collect();
    let report = run_with_capital(&spec, &candles, case.capital).expect("run");
    serde_json::to_string(&report).expect("serialize")
}

#[test]
fn golden_reports_match() {
    let dir = golden_dir();
    let bless = std::env::var("WICKRA_BLESS").is_ok();
    let mut cases: Vec<PathBuf> = fs::read_dir(dir.join("cases"))
        .expect("cases dir")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    cases.sort();
    assert!(!cases.is_empty(), "no golden cases found");

    for path in cases {
        let text = fs::read_to_string(&path).unwrap();
        let case: Case = serde_json::from_str(&text).unwrap();
        let got = run_case(&case);
        let expected_path = dir.join("expected").join(format!("{}.json", case.name));
        if bless {
            fs::create_dir_all(dir.join("expected")).unwrap();
            fs::write(&expected_path, format!("{got}\n")).unwrap();
            continue;
        }
        let want = fs::read_to_string(&expected_path).unwrap_or_else(|_| {
            panic!(
                "missing expected report for {} (run with WICKRA_BLESS=1)",
                case.name
            )
        });
        assert_eq!(got, want.trim_end(), "golden mismatch for {}", case.name);
    }
}
