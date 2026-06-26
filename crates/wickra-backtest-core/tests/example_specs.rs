//! Every strategy spec shipped under `examples/` and `examples/strategies/`
//! must parse and validate, so the documented cookbook strategies can never
//! drift out of sync with the spec DSL.

use std::fs;
use std::path::PathBuf;

use wickra_backtest_core::StrategySpec;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples")
}

#[test]
fn every_example_spec_parses() {
    let root = examples_dir();
    let mut specs: Vec<PathBuf> = Vec::new();
    // The top-level ema-cross.json plus every file under strategies/.
    specs.push(root.join("ema-cross.json"));
    for entry in fs::read_dir(root.join("strategies")).expect("strategies dir") {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|x| x == "json") {
            specs.push(path);
        }
    }
    specs.sort();
    assert!(specs.len() >= 6, "expected the cookbook strategy set");

    for path in specs {
        let text =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        StrategySpec::parse(&text)
            .unwrap_or_else(|e| panic!("spec {} failed to parse/validate: {e}", path.display()));
    }
}
