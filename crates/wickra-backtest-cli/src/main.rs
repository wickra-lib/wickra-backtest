//! `wkbt` — the Wickra backtester command line.
//!
//! ```text
//! wkbt run --data candles.csv --spec strategy.json [--capital N]
//!          [--report report.json] [--trades trades.jsonl] [--equity equity.jsonl]
//! ```

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde::Serialize;
use wickra_backtest_core::{run_with_capital, StrategySpec, DEFAULT_CAPITAL};
use wickra_backtest_data::load_candles;

#[derive(Parser)]
#[command(
    name = "wkbt",
    version,
    about = "Streaming-native backtester for Wickra strategies"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Run a backtest of a strategy spec over a data file.
    Run {
        /// Market-data file (CSV / JSONL / JSON).
        #[arg(long)]
        data: PathBuf,
        /// Strategy spec JSON file.
        #[arg(long)]
        spec: PathBuf,
        /// Starting capital.
        #[arg(long, default_value_t = DEFAULT_CAPITAL)]
        capital: f64,
        /// Write the full report as JSON to this path.
        #[arg(long)]
        report: Option<PathBuf>,
        /// Write the trade log as JSON Lines to this path.
        #[arg(long)]
        trades: Option<PathBuf>,
        /// Write the equity curve as JSON Lines to this path.
        #[arg(long)]
        equity: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    match dispatch(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn dispatch(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Run {
            data,
            spec,
            capital,
            report,
            trades,
            equity,
        } => {
            let candles = load_candles(&data).map_err(|e| e.to_string())?;
            let spec_json = std::fs::read_to_string(&spec)
                .map_err(|e| format!("reading {}: {e}", spec.display()))?;
            let strategy = StrategySpec::parse(&spec_json).map_err(|e| e.to_string())?;
            let result =
                run_with_capital(&strategy, &candles, capital).map_err(|e| e.to_string())?;

            let m = &result.metrics;
            println!("bars       {}", result.equity.len());
            println!("trades     {}", m.num_trades);
            println!("return     {:.2}%", m.return_pct);
            println!("pnl        {:.2}", m.pnl);
            println!("sharpe     {:.3}", m.sharpe);
            println!("max dd     {:.2}%", m.max_drawdown);
            println!("win rate   {:.1}%", m.win_rate);
            println!("fees       {:.2}", result.fees_paid);

            if let Some(path) = report {
                write_json(&path, &result)?;
            }
            if let Some(path) = trades {
                write_jsonl(&path, &result.trades)?;
            }
            if let Some(path) = equity {
                write_jsonl(&path, &result.equity)?;
            }
            Ok(())
        }
    }
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| format!("writing {}: {e}", path.display()))
}

fn write_jsonl<T: Serialize>(path: &Path, items: &[T]) -> Result<(), String> {
    let mut buf = String::new();
    for item in items {
        buf.push_str(&serde_json::to_string(item).map_err(|e| e.to_string())?);
        buf.push('\n');
    }
    std::fs::write(path, buf).map_err(|e| format!("writing {}: {e}", path.display()))
}
