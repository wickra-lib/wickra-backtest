//! # wickra-backtest-data
//!
//! Loaders that turn market-history files into the [`Candle`] stream the
//! [`wickra-backtest-core`] engine consumes. CSV (`time,open,high,low,close[,volume]`),
//! JSON Lines (one [`Candle`] object per line) and a JSON array are supported,
//! dispatched by file extension.

#![forbid(unsafe_code)]

use std::path::Path;

use wickra_backtest_core::{BacktestError, Candle, Result};

/// Load candles from a file, choosing the parser by extension
/// (`.jsonl`/`.ndjson` → JSON Lines, `.json` → JSON array, anything else → CSV).
pub fn load_candles(path: &Path) -> Result<Vec<Candle>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| BacktestError::InvalidData(format!("reading {}: {e}", path.display())))?;
    match path.extension().and_then(|e| e.to_str()) {
        Some("jsonl" | "ndjson") => parse_jsonl(&content),
        Some("json") => parse_json_array(&content),
        _ => parse_csv(&content),
    }
}

/// Parse CSV with columns `time,open,high,low,close[,volume]`. A non-numeric
/// first row is treated as a header and skipped.
pub fn parse_csv(content: &str) -> Result<Vec<Candle>> {
    let mut out = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split(',').map(str::trim).collect();
        // Skip a header row (first field not a number).
        if i == 0 && cols.first().is_some_and(|c| c.parse::<f64>().is_err()) {
            continue;
        }
        if cols.len() < 5 {
            return Err(BacktestError::InvalidData(format!(
                "CSV line {}: expected at least 5 columns (time,o,h,l,c), got {}",
                i + 1,
                cols.len()
            )));
        }
        let num = |idx: usize| -> Result<f64> {
            cols[idx].parse::<f64>().map_err(|_| {
                BacktestError::InvalidData(format!(
                    "CSV line {}: column {idx} is not a number",
                    i + 1
                ))
            })
        };
        out.push(Candle {
            time: num(0)? as i64,
            open: num(1)?,
            high: num(2)?,
            low: num(3)?,
            close: num(4)?,
            volume: if cols.len() > 5 { num(5)? } else { 0.0 },
        });
    }
    Ok(out)
}

/// Parse JSON Lines: one [`Candle`] object per non-empty line.
pub fn parse_jsonl(content: &str) -> Result<Vec<Candle>> {
    let mut out = Vec::new();
    for (i, line) in content.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let candle: Candle = serde_json::from_str(line)
            .map_err(|e| BacktestError::InvalidData(format!("JSONL line {}: {e}", i + 1)))?;
        out.push(candle);
    }
    Ok(out)
}

/// Parse a JSON array of [`Candle`] objects.
pub fn parse_json_array(content: &str) -> Result<Vec<Candle>> {
    serde_json::from_str(content)
        .map_err(|e| BacktestError::InvalidData(format!("JSON array: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_with_header_and_volume() {
        let csv = "time,open,high,low,close,volume\n1,10,12,9,11,100\n2,11,13,10,12,200\n";
        let c = parse_csv(csv).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].time, 1);
        assert!((c[0].close - 11.0).abs() < 1e-9);
        assert!((c[1].volume - 200.0).abs() < 1e-9);
    }

    #[test]
    fn csv_without_header_or_volume() {
        let csv = "1,10,12,9,11\n2,11,13,10,12\n";
        let c = parse_csv(csv).unwrap();
        assert_eq!(c.len(), 2);
        assert!((c[0].volume).abs() < f64::EPSILON);
    }

    #[test]
    fn csv_too_few_columns_errors() {
        assert!(parse_csv("1,2,3\n").is_err());
    }

    #[test]
    fn jsonl_roundtrip() {
        let jsonl = "{\"time\":1,\"open\":1,\"high\":2,\"low\":0.5,\"close\":1.5}\n{\"time\":2,\"open\":1.5,\"high\":2,\"low\":1,\"close\":1.8,\"volume\":5}\n";
        let c = parse_jsonl(jsonl).unwrap();
        assert_eq!(c.len(), 2);
        assert_eq!(c[1].time, 2);
        assert!((c[1].volume - 5.0).abs() < 1e-9);
    }

    #[test]
    fn json_array() {
        let json = "[{\"time\":1,\"open\":1,\"high\":2,\"low\":0.5,\"close\":1.5}]";
        let c = parse_json_array(json).unwrap();
        assert_eq!(c.len(), 1);
    }
}
