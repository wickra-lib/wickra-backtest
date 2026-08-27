//! Streaming golden parity: driving each shared case (`golden/cases/*.json`) one
//! bar at a time must reproduce the canonical report (`golden/expected/*.json`)
//! that the batch entry point produces.
//!
//! This is the anchor for the same assertion in every binding. It deliberately
//! compares against the *existing* expected reports rather than blessing its own:
//! a streaming run that needed its own baseline would mean the two paths had
//! diverged, which is exactly what this pins shut. There is therefore no
//! `WICKRA_BLESS` here -- `tests/golden.rs` owns the baseline.

use std::fs;
use std::path::PathBuf;

use serde::Deserialize;
use wickra_backtest_core::{Candle, StrategySpec, StreamingBacktest};

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

fn stream_case(case: &Case) -> String {
    let spec = StrategySpec::parse(&case.spec.to_string()).expect("spec parses");
    let mut bt = StreamingBacktest::new_owned(spec, case.capital).expect("start");
    for i in 0..case.close.len() {
        let candle = Candle {
            time: case.time[i],
            open: case.open[i],
            high: case.high[i],
            low: case.low[i],
            close: case.close[i],
            volume: case.volume[i],
        };
        bt.step(&candle).expect("step");
    }
    serde_json::to_string(&bt.finish()).expect("serialize")
}

#[test]
fn streaming_matches_the_golden_reports() {
    let dir = golden_dir();
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
        let got = stream_case(&case);
        let expected_path = dir.join("expected").join(format!("{}.json", case.name));
        let want = fs::read_to_string(&expected_path)
            .unwrap_or_else(|_| panic!("missing expected report for {}", case.name));
        assert_eq!(got, want.trim_end(), "streaming mismatch for {}", case.name);
    }
}
