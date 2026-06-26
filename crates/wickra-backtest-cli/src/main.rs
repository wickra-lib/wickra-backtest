//! `wkbt` — the Wickra backtester command line.
//!
//! ```text
//! wkbt run --data candles.csv --spec strategy.json [--capital N]
//!          [--resample-count N | --resample-interval I]
//!          [--report report.json] [--trades trades.jsonl] [--equity equity.jsonl]
//! wkbt schema   # print the strategy-spec JSON Schema
//! ```

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde::Serialize;
use wickra_backtest_core::{run_with_capital, strategy_spec_schema, StrategySpec, DEFAULT_CAPITAL};
use wickra_backtest_data::{load_candles, resample_by_count, resample_by_interval};

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
        /// Resample the data into fixed groups of this many bars before running.
        #[arg(long, conflicts_with = "resample_interval")]
        resample_count: Option<usize>,
        /// Resample the data by this timestamp interval before running.
        #[arg(long)]
        resample_interval: Option<i64>,
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
    /// Print the JSON Schema for the strategy spec.
    Schema,
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
        Command::Schema => {
            println!("{}", strategy_spec_schema());
            Ok(())
        }
        Command::Run {
            data,
            spec,
            capital,
            resample_count,
            resample_interval,
            report,
            trades,
            equity,
        } => {
            let mut candles = load_candles(&data).map_err(|e| e.to_string())?;
            if let Some(n) = resample_count {
                candles = resample_by_count(&candles, n).map_err(|e| e.to_string())?;
            } else if let Some(interval) = resample_interval {
                candles = resample_by_interval(&candles, interval).map_err(|e| e.to_string())?;
            }
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
