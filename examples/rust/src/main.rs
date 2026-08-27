//! Run the shared EMA-cross strategy from Rust, both ways.
//!
//! ```text
//! cargo run -p wickra-backtest-examples
//! ```
//!
//! Reads the same `examples/sample.csv` and `examples/ema-cross.json` every
//! other language example uses, runs the whole series at once, then feeds the
//! same bars one at a time and checks that the two agree. That equality is the
//! point of the library: a live loop is the streaming path with a socket in
//! place of the file, so a backtest is not a separate model of the strategy.

use std::path::PathBuf;
use std::process::ExitCode;

use wickra_backtest::{run_with_capital, StrategySpec, StreamingBacktest};

const CAPITAL: f64 = 10_000.0;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn main() -> ExitCode {
    let dir = examples_dir();
    let candles = match wickra_backtest::data::load_candles(&dir.join("sample.csv")) {
        Ok(candles) => candles,
        Err(err) => {
            eprintln!("could not read sample.csv: {err}");
            return ExitCode::FAILURE;
        }
    };

    let spec_text = match std::fs::read_to_string(dir.join("ema-cross.json")) {
        Ok(text) => text,
        Err(err) => {
            eprintln!("could not read ema-cross.json: {err}");
            return ExitCode::FAILURE;
        }
    };
    let spec = match StrategySpec::parse(&spec_text) {
        Ok(spec) => spec,
        Err(err) => {
            eprintln!("spec rejected: {err}");
            return ExitCode::FAILURE;
        }
    };

    let batch = run_with_capital(&spec, &candles, CAPITAL).expect("the spec parsed, so it runs");

    // The same run, driven bar by bar. Replace the loop with reads from a socket
    // and this is a live strategy; nothing else about it changes.
    let mut live = StreamingBacktest::new(&spec, CAPITAL).expect("the spec parsed, so it starts");
    for candle in &candles {
        live.step(candle)
            .expect("the spec parsed, so every bar is accepted");
    }
    let streamed = live.finish();

    let metrics = &streamed.metrics;
    println!("bars            {}", candles.len());
    println!("trades          {}", metrics.num_trades);
    println!("pnl             {:.2}", metrics.pnl);
    println!("return %        {:.2}", metrics.return_pct);
    println!("max drawdown    {:.4}", metrics.max_drawdown);
    if let Some(last) = streamed.equity.last() {
        println!("final equity    {:.2}", last.equity);
    }

    // Compared through their serialised form: that is the shape every binding
    // hands back, so this is the same comparison they all make.
    let a = serde_json::to_string(&streamed).expect("report serialises");
    let b = serde_json::to_string(&batch).expect("report serialises");
    if a != b {
        eprintln!("streaming and batch disagree -- that should be impossible");
        return ExitCode::FAILURE;
    }
    println!("\nstreaming reproduces the batch report exactly");
    ExitCode::SUCCESS
}
