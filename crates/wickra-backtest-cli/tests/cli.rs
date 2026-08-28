//! End-to-end tests for the `wkbt` binary.
//!
//! The unit tests beside the source cover the helpers; these run the built
//! executable, which is the only way to reach argument parsing, the mutually
//! exclusive transform group, the exit codes and what actually lands on stdout.
//!
//! Everything a test needs is written into a temporary directory here rather
//! than read from the repository. `tests/` ships inside the published crate, so
//! a path reaching out to `examples/` would resolve in this tree and nowhere
//! else -- which is exactly how a test can pass for years and fail the moment
//! someone runs it from the packaged crate.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Enough bars for two EMAs to warm up and cross more than once.
fn write_candles(path: &Path) {
    use std::fmt::Write as _;
    let mut csv = String::from("time,open,high,low,close,volume\n");
    for i in 0..120_i64 {
        let mid = 100.0 + f64::sin(i as f64 * 0.25) * 8.0;
        let open = 100.0 + f64::sin((i as f64 - 1.0) * 0.25) * 8.0;
        let (high, low) = (mid.max(open) + 0.5, mid.min(open) - 0.5);
        let _ = writeln!(csv, "{i},{open:.4},{high:.4},{low:.4},{mid:.4},1000");
    }
    std::fs::write(path, csv).unwrap();
}

const SPEC: &str = r#"{
  "symbol": "TEST",
  "timeframe": "1h",
  "indicators": {
    "ema_fast": { "type": "Ema", "params": [5] },
    "ema_slow": { "type": "Ema", "params": [15] }
  },
  "entry": { "cross_above": ["ema_fast", "ema_slow"] },
  "exit": { "cross_below": ["ema_fast", "ema_slow"] },
  "sizing": { "type": "fixed_fraction", "fraction": 0.95 }
}"#;

/// A fresh directory per test, so tests running in parallel cannot collide and
/// a failure leaves its inputs behind to look at.
struct Fixture {
    dir: PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("wkbt_cli_{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        write_candles(&dir.join("data.csv"));
        std::fs::write(dir.join("spec.json"), SPEC).unwrap();
        Self { dir }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    fn wkbt(args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_wkbt"))
            .args(args)
            .output()
            .expect("wkbt should be runnable")
    }

    /// Run `wkbt run` over this fixture's data and spec, plus any extra flags.
    fn run(&self, extra: &[&str]) -> Output {
        let data = self.path("data.csv");
        let spec = self.path("spec.json");
        let mut args = vec![
            "run",
            "--data",
            data.to_str().unwrap(),
            "--spec",
            spec.to_str().unwrap(),
        ];
        args.extend_from_slice(extra);
        Self::wkbt(&args)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

#[test]
fn schema_prints_the_committed_strategy_spec_schema() {
    let output = Fixture::wkbt(&["schema"]);
    assert!(output.status.success(), "{}", stderr(&output));
    let parsed: serde_json::Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert!(
        parsed.get("$schema").is_some(),
        "not a JSON Schema document"
    );
    assert!(stdout(&output).contains("StrategySpec"));
}

#[test]
fn run_reports_the_summary_a_reader_expects() {
    let fixture = Fixture::new("summary");
    let output = fixture.run(&[]);
    assert!(output.status.success(), "{}", stderr(&output));
    let text = stdout(&output);
    for label in [
        "bars", "trades", "return", "pnl", "sharpe", "max dd", "win rate", "fees",
    ] {
        assert!(
            text.contains(label),
            "summary is missing `{label}`:\n{text}"
        );
    }
}

#[test]
fn run_writes_every_output_file_it_is_given() {
    let fixture = Fixture::new("outputs");
    let report = fixture.path("report.json");
    let trades = fixture.path("trades.jsonl");
    let equity = fixture.path("equity.jsonl");
    let output = fixture.run(&[
        "--report",
        report.to_str().unwrap(),
        "--trades",
        trades.to_str().unwrap(),
        "--equity",
        equity.to_str().unwrap(),
    ]);
    assert!(output.status.success(), "{}", stderr(&output));

    let parsed: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
    assert!(parsed.get("metrics").is_some(), "report has no metrics");

    // JSON Lines: every line has to parse on its own, which is the whole point
    // of the format and the thing a stray pretty-print would break.
    for path in [&trades, &equity] {
        let text = std::fs::read_to_string(path).unwrap();
        for line in text.lines() {
            serde_json::from_str::<serde_json::Value>(line).unwrap();
        }
    }
    assert!(!std::fs::read_to_string(&equity).unwrap().is_empty());
}

#[test]
fn streaming_and_batch_produce_the_same_run() {
    // The claim the whole project rests on, checked at the command line: the
    // same data through `--stream` and through the batch path must agree, in
    // the report and in the equity curve written beside it.
    let fixture = Fixture::new("stream_equals_batch");
    let batch_report = fixture.path("batch.json");
    let batch_equity = fixture.path("batch_equity.jsonl");
    let stream_report = fixture.path("stream.json");
    let stream_equity = fixture.path("stream_equity.jsonl");

    let batch = fixture.run(&[
        "--report",
        batch_report.to_str().unwrap(),
        "--equity",
        batch_equity.to_str().unwrap(),
    ]);
    assert!(batch.status.success(), "{}", stderr(&batch));

    let streamed = fixture.run(&[
        "--stream",
        "--report",
        stream_report.to_str().unwrap(),
        "--equity",
        stream_equity.to_str().unwrap(),
    ]);
    assert!(streamed.status.success(), "{}", stderr(&streamed));

    assert_eq!(
        std::fs::read_to_string(&batch_report).unwrap(),
        std::fs::read_to_string(&stream_report).unwrap(),
        "the streamed report differs from the batch one"
    );
    // The streamed equity file is written incrementally, bar by bar, while the
    // batch one is written in a single pass at the end. Same bytes either way.
    assert_eq!(
        std::fs::read_to_string(&batch_equity).unwrap(),
        std::fs::read_to_string(&stream_equity).unwrap(),
        "the incrementally written equity curve differs from the batch one"
    );
    assert_eq!(stdout(&batch), stdout(&streamed));
}

/// The bar count the summary reports, so a transform can be shown to have done
/// something rather than merely to have been accepted.
fn bars_in(output: &Output) -> usize {
    stdout(output)
        .lines()
        .find_map(|line| line.strip_prefix("bars"))
        .expect("summary reports a bar count")
        .trim()
        .parse()
        .expect("bar count is a number")
}

#[test]
fn each_transform_changes_the_series_it_is_given() {
    let fixture = Fixture::new("transforms");
    let plain = bars_in(&fixture.run(&[]));

    // Grouping four bars into one must produce roughly a quarter as many.
    let resampled = bars_in(&fixture.run(&["--resample-count", "4"]));
    assert!(
        resampled < plain && resampled >= plain / 4,
        "resample-count produced {resampled} bars from {plain}"
    );

    // The alternative bar types are event-driven: they emit a bar per price
    // move of the configured size, so the count is not a function of the input
    // length. Asserting only that they ran and produced something.
    for args in [
        vec!["--renko", "2.0"],
        vec!["--kagi", "2.0"],
        vec!["--pnf", "1.0:3"],
    ] {
        let output = fixture.run(&args);
        assert!(output.status.success(), "{:?}: {}", args, stderr(&output));
        assert!(bars_in(&output) > 0, "{args:?} produced an empty series");
    }
}

#[test]
fn two_transforms_at_once_are_refused() {
    // They are mutually exclusive by an ArgGroup, and a group that stops
    // matching its arguments is the failure clap_definition_is_valid cannot see.
    let fixture = Fixture::new("transform_conflict");
    let output = fixture.run(&["--renko", "2.0", "--kagi", "2.0"]);
    assert!(!output.status.success(), "two transforms were accepted");
    assert!(
        stderr(&output).contains("cannot be used with"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn a_missing_data_file_names_the_path() {
    let fixture = Fixture::new("missing_data");
    let spec = fixture.path("spec.json");
    let output = Fixture::wkbt(&[
        "run",
        "--data",
        fixture.path("absent.csv").to_str().unwrap(),
        "--spec",
        spec.to_str().unwrap(),
    ]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("absent.csv"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn an_invalid_spec_fails_with_a_message_rather_than_a_panic() {
    let fixture = Fixture::new("bad_spec");
    std::fs::write(fixture.path("spec.json"), "{\"symbol\": \"TEST\"").unwrap();
    let output = fixture.run(&[]);
    assert!(!output.status.success());
    let text = stderr(&output);
    assert!(text.starts_with("error: "), "{text}");
    assert!(!text.contains("panicked"), "{text}");
}

#[test]
fn an_unparseable_pnf_argument_explains_the_format() {
    let fixture = Fixture::new("bad_pnf");
    let output = fixture.run(&["--pnf", "1.0"]);
    assert!(!output.status.success());
    assert!(
        stderr(&output).contains("BOX:REVERSAL"),
        "{}",
        stderr(&output)
    );
}

#[test]
fn run_requires_both_data_and_spec() {
    let fixture = Fixture::new("missing_args");
    let output = Fixture::wkbt(&["run", "--data", fixture.path("data.csv").to_str().unwrap()]);
    assert!(!output.status.success());
    assert!(stderr(&output).contains("--spec"), "{}", stderr(&output));
}

#[test]
fn capital_reaches_the_engine() {
    // The default and an explicit value must not produce the same report, or
    // --capital is being parsed and dropped.
    let fixture = Fixture::new("capital");
    let default_report = fixture.path("default.json");
    let doubled_report = fixture.path("doubled.json");

    fixture.run(&["--report", default_report.to_str().unwrap()]);
    fixture.run(&[
        "--capital",
        "20000",
        "--report",
        doubled_report.to_str().unwrap(),
    ]);

    assert_ne!(
        std::fs::read_to_string(&default_report).unwrap(),
        std::fs::read_to_string(&doubled_report).unwrap(),
        "--capital had no effect on the report"
    );
}
