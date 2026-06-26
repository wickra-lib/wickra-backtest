//! Feed golden parity anchor: the Rust engine turns each shared feed request
//! (`golden/requests/*.json`) into the canonical expected report
//! (`golden/expected_json/*.json`) through the unified `run_json` entry point.
//! Every language binding asserts its own `run_json` output against the same
//! expected reports, so cross-language equality is pinned for the
//! microstructure feed paths (derivatives, order book, trades, cross-section and
//! the pairwise reference series), not just the plain OHLCV path.
//!
//! Regenerate the expected reports after an intentional engine change:
//!
//! ```text
//! WICKRA_BLESS=1 cargo test -p wickra-backtest-core --test golden_json
//! ```

use std::fs;
use std::path::PathBuf;

use wickra_backtest_core::run_json;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../golden")
}

#[test]
fn golden_json_reports_match() {
    let dir = golden_dir();
    let bless = std::env::var("WICKRA_BLESS").is_ok();
    let mut requests: Vec<PathBuf> = fs::read_dir(dir.join("requests"))
        .expect("requests dir")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    requests.sort();
    assert!(!requests.is_empty(), "no golden requests found");

    for path in requests {
        let name = path.file_stem().unwrap().to_str().unwrap().to_string();
        let request = fs::read_to_string(&path).unwrap();
        let got = run_json(&request).expect("run_json");
        let expected_path = dir.join("expected_json").join(format!("{name}.json"));
        if bless {
            fs::create_dir_all(dir.join("expected_json")).unwrap();
            fs::write(&expected_path, format!("{got}\n")).unwrap();
            continue;
        }
        let want = fs::read_to_string(&expected_path).unwrap_or_else(|_| {
            panic!("missing expected report for {name} (run with WICKRA_BLESS=1)")
        });
        assert_eq!(got, want.trim_end(), "golden mismatch for {name}");
    }
}
