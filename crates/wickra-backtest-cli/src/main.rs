//! `wkbt` — the Wickra backtester command line.
//!
//! ```text
//! wkbt run --data candles.csv --spec strategy.json [--capital N]
//!          [--resample-count N | --resample-interval I |
//!           --renko BOX | --kagi REVERSAL | --pnf BOX:REVERSAL]
//!          [--report report.json] [--trades trades.jsonl] [--equity equity.jsonl]
//!          [--stream]
//! wkbt fetch --symbol BTCUSDT --interval 1h --limit 500 --out data.csv  # (binance feature)
//! wkbt schema   # print the strategy-spec JSON Schema
//! ```

#![forbid(unsafe_code)]

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde::Serialize;
use wickra_backtest_core::{
    run_stream, run_with_capital, strategy_spec_schema, StrategySpec, DEFAULT_CAPITAL,
};
use wickra_backtest_data::{
    load_candles, resample_by_count, resample_by_interval, to_kagi, to_pnf, to_renko,
};

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
// The `Run` variant carries all the run options and is naturally much larger
// than the unit `Schema` variant; this is a short-lived CLI command parsed once,
// so the size difference is irrelevant and boxing clap arg structs is awkward.
#[allow(clippy::large_enum_variant)]
enum Command {
    /// Run a backtest of a strategy spec over a data file.
    #[command(group(clap::ArgGroup::new("transform")
        .args(["resample_count", "resample_interval", "renko", "kagi", "pnf"])))]
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
        #[arg(long)]
        resample_count: Option<usize>,
        /// Resample the data by this timestamp interval before running.
        #[arg(long)]
        resample_interval: Option<i64>,
        /// Rebuild the data as Renko bricks of this box size before running.
        #[arg(long)]
        renko: Option<f64>,
        /// Rebuild the data as Kagi segments with this reversal amount.
        #[arg(long)]
        kagi: Option<f64>,
        /// Rebuild the data as Point-and-Figure columns (`BOX:REVERSAL`, e.g. `1.0:3`).
        #[arg(long)]
        pnf: Option<String>,
        /// Write the full report as JSON to this path.
        #[arg(long)]
        report: Option<PathBuf>,
        /// Write the trade log as JSON Lines to this path.
        #[arg(long)]
        trades: Option<PathBuf>,
        /// Write the equity curve as JSON Lines to this path.
        #[arg(long)]
        equity: Option<PathBuf>,
        /// Drive the engine one bar at a time (the live/streaming path) and emit
        /// each equity point to `--equity` as it is produced, instead of in one
        /// batch. The result is identical; this exercises the live code path.
        #[arg(long)]
        stream: bool,
    },
    /// Fetch historical candles from Binance and write them to a file
    /// (requires the `binance` build feature).
    #[cfg(feature = "binance")]
    Fetch {
        /// Trading symbol, e.g. `BTCUSDT`.
        #[arg(long)]
        symbol: String,
        /// Binance interval, e.g. `1m`, `1h`, `1d`.
        #[arg(long, default_value = "1h")]
        interval: String,
        /// Number of bars (Binance caps this at 1000).
        #[arg(long, default_value_t = 500)]
        limit: u32,
        /// Output file (`.csv` / `.jsonl` / `.json` by extension).
        #[arg(long)]
        out: PathBuf,
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
        #[cfg(feature = "binance")]
        Command::Fetch {
            symbol,
            interval,
            limit,
            out,
        } => {
            let candles = wickra_backtest_data::fetch_klines(&symbol, &interval, limit)
                .map_err(|e| e.to_string())?;
            write_candles(&out, &candles)?;
            println!("fetched {} bars -> {}", candles.len(), out.display());
            Ok(())
        }
        Command::Run {
            data,
            spec,
            capital,
            resample_count,
            resample_interval,
            renko,
            kagi,
            pnf,
            report,
            trades,
            equity,
            stream,
        } => {
            let mut candles = load_candles(&data).map_err(|e| e.to_string())?;
            // At most one transform applies (enforced by the `transform` group).
            if let Some(n) = resample_count {
                candles = resample_by_count(&candles, n).map_err(|e| e.to_string())?;
            } else if let Some(interval) = resample_interval {
                candles = resample_by_interval(&candles, interval).map_err(|e| e.to_string())?;
            } else if let Some(box_size) = renko {
                candles = to_renko(&candles, box_size).map_err(|e| e.to_string())?;
            } else if let Some(reversal) = kagi {
                candles = to_kagi(&candles, reversal).map_err(|e| e.to_string())?;
            } else if let Some(spec) = pnf {
                let (box_size, reversal) = parse_pnf(&spec)?;
                candles = to_pnf(&candles, box_size, reversal).map_err(|e| e.to_string())?;
            }
            let spec_json = std::fs::read_to_string(&spec)
                .map_err(|e| format!("reading {}: {e}", spec.display()))?;
            let strategy = StrategySpec::parse(&spec_json).map_err(|e| e.to_string())?;

            // In `--stream` mode the engine is driven one bar at a time (the live
            // code path) and each equity point is written to `--equity` as it is
            // produced; the resulting report is identical to the batch run.
            let equity_streamed = stream && equity.is_some();
            let result = if stream {
                let mut writer = match &equity {
                    Some(path) => Some(open_jsonl(path)?),
                    None => None,
                };
                let mut write_err: Option<String> = None;
                let r = run_stream(&strategy, &candles, capital, |_, bt| {
                    if write_err.is_some() {
                        return;
                    }
                    if let (Some(w), Some(point)) = (writer.as_mut(), bt.latest_equity()) {
                        if let Err(e) = write_jsonl_line(w, &point) {
                            write_err = Some(e);
                        }
                    }
                })
                .map_err(|e| e.to_string())?;
                if let Some(e) = write_err {
                    return Err(e);
                }
                if let Some(mut w) = writer {
                    w.flush()
                        .map_err(|e| format!("flushing equity stream: {e}"))?;
                }
                r
            } else {
                run_with_capital(&strategy, &candles, capital).map_err(|e| e.to_string())?
            };

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
            // In stream mode the equity file was already written incrementally.
            if let Some(path) = equity {
                if !equity_streamed {
                    write_jsonl(&path, &result.equity)?;
                }
            }
            Ok(())
        }
    }
}

/// Parse the `--pnf BOX:REVERSAL` argument (e.g. `1.0:3`).
fn parse_pnf(arg: &str) -> Result<(f64, usize), String> {
    let (box_str, rev_str) = arg
        .split_once(':')
        .ok_or_else(|| format!("--pnf expects BOX:REVERSAL (e.g. 1.0:3), got `{arg}`"))?;
    let box_size = box_str
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("--pnf box size `{box_str}` is not a number"))?;
    let reversal = rev_str
        .trim()
        .parse::<usize>()
        .map_err(|_| format!("--pnf reversal `{rev_str}` is not an integer"))?;
    Ok((box_size, reversal))
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

/// Write candles to a file, choosing the format by extension
/// (`.jsonl`/`.ndjson` → JSON Lines, `.json` → JSON array, else CSV).
#[cfg(feature = "binance")]
fn write_candles(path: &Path, candles: &[wickra_backtest_core::Candle]) -> Result<(), String> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("jsonl" | "ndjson") => write_jsonl(path, candles),
        Some("json") => write_json(path, &candles),
        _ => {
            use std::fmt::Write as _;
            let mut buf = String::from("time,open,high,low,close,volume\n");
            for c in candles {
                let _ = writeln!(
                    buf,
                    "{},{},{},{},{},{}",
                    c.time, c.open, c.high, c.low, c.close, c.volume
                );
            }
            std::fs::write(path, buf).map_err(|e| format!("writing {}: {e}", path.display()))
        }
    }
}

/// Open a JSON Lines file for incremental (streaming) writes.
fn open_jsonl(path: &Path) -> Result<BufWriter<File>, String> {
    File::create(path)
        .map(BufWriter::new)
        .map_err(|e| format!("creating {}: {e}", path.display()))
}

/// Append one item as a JSON line to an open writer.
fn write_jsonl_line<T: Serialize>(writer: &mut BufWriter<File>, item: &T) -> Result<(), String> {
    let line = serde_json::to_string(item).map_err(|e| e.to_string())?;
    writeln!(writer, "{line}").map_err(|e| format!("writing equity stream: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// A unique path under the system temp directory, so tests running in
    /// parallel cannot collide on a filename.
    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("wkbt_cli_test_{name}"))
    }

    #[test]
    fn clap_definition_is_valid() {
        // clap's own sanity check: duplicate long names, a group naming an
        // argument that does not exist, a default that fails to parse. All of
        // them panic at run time on the user's first invocation otherwise.
        Cli::command().debug_assert();
    }

    #[test]
    fn parse_pnf_accepts_box_and_reversal() {
        assert_eq!(parse_pnf("1.0:3"), Ok((1.0, 3)));
        assert_eq!(parse_pnf("0.5:2"), Ok((0.5, 2)));
    }

    #[test]
    fn parse_pnf_tolerates_surrounding_space() {
        assert_eq!(parse_pnf(" 1.5 : 4 "), Ok((1.5, 4)));
    }

    #[test]
    fn parse_pnf_rejects_a_missing_separator() {
        let err = parse_pnf("1.0").unwrap_err();
        assert!(err.contains("BOX:REVERSAL"), "{err}");
    }

    #[test]
    fn parse_pnf_rejects_a_non_numeric_box() {
        let err = parse_pnf("wide:3").unwrap_err();
        assert!(err.contains("box size"), "{err}");
    }

    #[test]
    fn parse_pnf_rejects_a_fractional_reversal() {
        // The reversal is a count of boxes, so 2.5 is not a smaller reversal --
        // it is a typo, and saying so beats truncating it silently.
        let err = parse_pnf("1.0:2.5").unwrap_err();
        assert!(err.contains("reversal"), "{err}");
    }

    #[test]
    fn write_json_round_trips() {
        let path = temp_path("write_json.json");
        write_json(&path, &vec![1_u32, 2, 3]).unwrap();
        let back: Vec<u32> =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(back, vec![1, 2, 3]);
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn write_jsonl_writes_one_object_per_line() {
        let path = temp_path("write_jsonl.jsonl");
        write_jsonl(&path, &[1_u32, 2, 3]).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().collect::<Vec<_>>(), ["1", "2", "3"]);
        // Trailing newline: a JSON Lines file that ends mid-line is malformed.
        assert!(text.ends_with('\n'));
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn write_jsonl_on_an_empty_slice_writes_an_empty_file() {
        let path = temp_path("write_jsonl_empty.jsonl");
        write_jsonl(&path, &[] as &[u32]).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "");
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn incremental_jsonl_matches_the_batch_form() {
        // The streaming equity path writes line by line through these two; the
        // batch path writes the whole slice at once. Both files must be the
        // same, because `--stream` is documented as producing an identical run.
        let incremental = temp_path("jsonl_incremental.jsonl");
        let batch = temp_path("jsonl_batch.jsonl");
        let items = [10_u32, 20, 30];

        let mut writer = open_jsonl(&incremental).unwrap();
        for item in &items {
            write_jsonl_line(&mut writer, item).unwrap();
        }
        writer.flush().unwrap();
        write_jsonl(&batch, &items).unwrap();

        assert_eq!(
            std::fs::read_to_string(&incremental).unwrap(),
            std::fs::read_to_string(&batch).unwrap()
        );
        std::fs::remove_file(&incremental).unwrap();
        std::fs::remove_file(&batch).unwrap();
    }

    #[test]
    fn write_json_reports_the_path_it_could_not_write() {
        // A directory that does not exist is the common typo; the message has to
        // name the path or the user is left guessing which argument was wrong.
        let path = temp_path("no_such_dir").join("report.json");
        let err = write_json(&path, &1_u32).unwrap_err();
        assert!(err.contains("no_such_dir"), "{err}");
    }

    #[cfg(feature = "binance")]
    #[test]
    fn write_candles_picks_the_format_from_the_extension() {
        use wickra_backtest_core::Candle;
        let candles = [Candle {
            time: 1,
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: 10.0,
        }];

        let csv = temp_path("candles.csv");
        write_candles(&csv, &candles).unwrap();
        let text = std::fs::read_to_string(&csv).unwrap();
        assert!(
            text.starts_with("time,open,high,low,close,volume\n"),
            "{text}"
        );
        assert!(text.contains("1,1,2,0.5,1.5,10"), "{text}");

        let jsonl = temp_path("candles.jsonl");
        write_candles(&jsonl, &candles).unwrap();
        assert_eq!(std::fs::read_to_string(&jsonl).unwrap().lines().count(), 1);

        let json = temp_path("candles.json");
        write_candles(&json, &candles).unwrap();
        let parsed: Vec<Candle> =
            serde_json::from_str(&std::fs::read_to_string(&json).unwrap()).unwrap();
        assert_eq!(parsed.len(), 1);

        for path in [csv, jsonl, json] {
            std::fs::remove_file(&path).unwrap();
        }
    }
}
