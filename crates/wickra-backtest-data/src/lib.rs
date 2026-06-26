//! # wickra-backtest-data
//!
//! Loaders that turn market-history files into the [`Candle`] stream the
//! [`wickra-backtest-core`] engine consumes. CSV (`time,open,high,low,close[,volume]`),
//! JSON Lines (one [`Candle`] object per line) and a JSON array are supported,
//! dispatched by file extension; Apache Parquet is supported behind the
//! `parquet` feature. Candles can also be resampled to a coarser timeframe by a
//! fixed bar count or a timestamp interval.

#![forbid(unsafe_code)]

use std::path::Path;

use wickra_backtest_core::{BacktestError, Candle, Result};

/// Load candles from a file, choosing the parser by extension
/// (`.jsonl`/`.ndjson` → JSON Lines, `.json` → JSON array, `.parquet` → Parquet
/// when the `parquet` feature is on, anything else → CSV).
pub fn load_candles(path: &Path) -> Result<Vec<Candle>> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("parquet") => load_parquet(path),
        Some("jsonl" | "ndjson") => parse_jsonl(&read_text(path)?),
        Some("json") => parse_json_array(&read_text(path)?),
        _ => parse_csv(&read_text(path)?),
    }
}

fn read_text(path: &Path) -> Result<String> {
    std::fs::read_to_string(path)
        .map_err(|e| BacktestError::InvalidData(format!("reading {}: {e}", path.display())))
}

/// Load candles from an Apache Parquet file with columns
/// `time, open, high, low, close[, volume]` (matched case-insensitively).
/// Integer or floating-point columns are both accepted. Requires the `parquet`
/// feature.
#[cfg(not(feature = "parquet"))]
pub fn load_parquet(_path: &Path) -> Result<Vec<Candle>> {
    Err(BacktestError::InvalidData(
        "Parquet support is not compiled in; rebuild with the `parquet` feature".into(),
    ))
}

/// Load candles from an Apache Parquet file with columns
/// `time, open, high, low, close[, volume]` (matched case-insensitively).
/// Integer or floating-point columns are both accepted.
#[cfg(feature = "parquet")]
pub fn load_parquet(path: &Path) -> Result<Vec<Candle>> {
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;

    let file = std::fs::File::open(path)
        .map_err(|e| BacktestError::InvalidData(format!("opening {}: {e}", path.display())))?;
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|e| BacktestError::InvalidData(format!("parquet: {e}")))?;

    let schema = builder.schema().clone();
    let col = |name: &str| {
        schema
            .fields()
            .iter()
            .position(|f| f.name().eq_ignore_ascii_case(name))
    };
    let require = |name: &str| {
        col(name).ok_or_else(|| {
            BacktestError::InvalidData(format!("parquet: missing required column `{name}`"))
        })
    };
    let (i_time, i_open, i_high, i_low, i_close) = (
        require("time")?,
        require("open")?,
        require("high")?,
        require("low")?,
        require("close")?,
    );
    let i_volume = col("volume");

    let reader = builder
        .build()
        .map_err(|e| BacktestError::InvalidData(format!("parquet: {e}")))?;

    let mut out = Vec::new();
    for batch in reader {
        let batch = batch.map_err(|e| BacktestError::InvalidData(format!("parquet: {e}")))?;
        let time = column_i64(&batch, i_time)?;
        let open = column_f64(&batch, i_open)?;
        let high = column_f64(&batch, i_high)?;
        let low = column_f64(&batch, i_low)?;
        let close = column_f64(&batch, i_close)?;
        let volume = i_volume.map(|i| column_f64(&batch, i)).transpose()?;
        for r in 0..batch.num_rows() {
            out.push(Candle {
                time: time[r],
                open: open[r],
                high: high[r],
                low: low[r],
                close: close[r],
                volume: volume.as_ref().map_or(0.0, |v| v[r]),
            });
        }
    }
    Ok(out)
}

/// Read an Arrow column as `f64`, accepting 32/64-bit int or float encodings.
#[cfg(feature = "parquet")]
fn column_f64(batch: &arrow_array::RecordBatch, idx: usize) -> Result<Vec<f64>> {
    use arrow_array::{
        cast::AsArray,
        types::{Float32Type, Float64Type, Int32Type, Int64Type},
    };

    let array = batch.column(idx);
    if array.null_count() > 0 {
        return Err(BacktestError::InvalidData(
            "parquet: null values are not allowed in OHLCV columns".into(),
        ));
    }
    let dt = array.data_type();
    if let Some(a) = array.as_primitive_opt::<Float64Type>() {
        Ok(a.values().to_vec())
    } else if let Some(a) = array.as_primitive_opt::<Float32Type>() {
        Ok(a.values().iter().map(|&v| f64::from(v)).collect())
    } else if let Some(a) = array.as_primitive_opt::<Int64Type>() {
        Ok(a.values().iter().map(|&v| v as f64).collect())
    } else if let Some(a) = array.as_primitive_opt::<Int32Type>() {
        Ok(a.values().iter().map(|&v| f64::from(v)).collect())
    } else {
        Err(BacktestError::InvalidData(format!(
            "parquet: column {idx} has unsupported numeric type {dt:?}"
        )))
    }
}

/// Read an Arrow column as `i64`, accepting 32/64-bit int or float encodings.
#[cfg(feature = "parquet")]
fn column_i64(batch: &arrow_array::RecordBatch, idx: usize) -> Result<Vec<i64>> {
    Ok(column_f64(batch, idx)?
        .into_iter()
        .map(|v| v as i64)
        .collect())
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

/// Aggregate one [`Candle`] from a non-empty slice of finer candles: the bucket
/// opens at the first open, closes at the last close, takes the extreme high/low
/// and sums the volume. The timestamp is the first candle's time.
fn aggregate(bucket: &[Candle]) -> Candle {
    let first = &bucket[0];
    let last = &bucket[bucket.len() - 1];
    let mut high = first.high;
    let mut low = first.low;
    let mut volume = 0.0;
    for c in bucket {
        high = high.max(c.high);
        low = low.min(c.low);
        volume += c.volume;
    }
    Candle {
        time: first.time,
        open: first.open,
        high,
        low,
        close: last.close,
        volume,
    }
}

/// Resample candles into fixed groups of `count` (e.g. five 1-minute bars into
/// one 5-minute bar). A trailing partial group is aggregated as-is. `count` must
/// be non-zero.
pub fn resample_by_count(candles: &[Candle], count: usize) -> Result<Vec<Candle>> {
    if count == 0 {
        return Err(BacktestError::InvalidData(
            "resample count must be > 0".into(),
        ));
    }
    Ok(candles.chunks(count).map(aggregate).collect())
}

/// Resample candles by a timestamp `interval`: candles whose `time` falls in the
/// same `floor(time / interval)` bucket are aggregated, and the bucket adopts
/// the floored start time. Input must be time-ordered; `interval` must be
/// non-zero.
pub fn resample_by_interval(candles: &[Candle], interval: i64) -> Result<Vec<Candle>> {
    if interval <= 0 {
        return Err(BacktestError::InvalidData(
            "resample interval must be > 0".into(),
        ));
    }
    let mut out: Vec<Candle> = Vec::new();
    let mut start = 0usize;
    for i in 0..candles.len() {
        let bucket = candles[i].time.div_euclid(interval);
        let next_bucket = candles
            .get(i + 1)
            .map(|c| c.time.div_euclid(interval) != bucket);
        if next_bucket != Some(false) {
            // i is the last candle of its bucket (or the final candle).
            let mut bar = aggregate(&candles[start..=i]);
            bar.time = bucket * interval;
            out.push(bar);
            start = i + 1;
        }
    }
    Ok(out)
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

    fn four_bars() -> Vec<Candle> {
        // time,open,high,low,close,volume
        parse_csv("0,10,12,9,11,100\n1,11,13,10,12,200\n2,12,14,11,13,300\n3,13,15,12,14,400\n")
            .unwrap()
    }

    #[test]
    fn resample_by_count_aggregates_buckets() {
        let bars = resample_by_count(&four_bars(), 2).unwrap();
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].time, 0);
        assert!((bars[0].open - 10.0).abs() < 1e-9);
        assert!((bars[0].high - 13.0).abs() < 1e-9); // max(12, 13)
        assert!((bars[0].low - 9.0).abs() < 1e-9); // min(9, 10)
        assert!((bars[0].close - 12.0).abs() < 1e-9); // last close
        assert!((bars[0].volume - 300.0).abs() < 1e-9); // 100 + 200
        assert!((bars[1].close - 14.0).abs() < 1e-9);
        assert!((bars[1].volume - 700.0).abs() < 1e-9);
    }

    #[test]
    fn resample_by_count_keeps_trailing_partial_group() {
        let bars = resample_by_count(&four_bars(), 3).unwrap();
        assert_eq!(bars.len(), 2); // [0,1,2] then [3]
        assert!((bars[1].close - 14.0).abs() < 1e-9);
        assert!((bars[1].volume - 400.0).abs() < 1e-9);
    }

    #[test]
    fn resample_by_interval_buckets_on_time() {
        let bars = resample_by_interval(&four_bars(), 2).unwrap();
        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].time, 0); // floor(0/2)*2 and floor(1/2)*2 == 0
        assert!((bars[0].close - 12.0).abs() < 1e-9);
        assert_eq!(bars[1].time, 2); // floor(2/2)*2 == 2
        assert!((bars[1].close - 14.0).abs() < 1e-9);
        assert!((bars[1].volume - 700.0).abs() < 1e-9);
    }

    #[test]
    fn resample_rejects_zero_step() {
        assert!(resample_by_count(&four_bars(), 0).is_err());
        assert!(resample_by_interval(&four_bars(), 0).is_err());
    }

    #[cfg(feature = "parquet")]
    #[test]
    fn parquet_round_trip() {
        use std::sync::Arc;

        use arrow_array::{Float64Array, Int64Array, RecordBatch};
        use arrow_schema::{DataType, Field, Schema};
        use parquet::arrow::ArrowWriter;

        let schema = Arc::new(Schema::new(vec![
            Field::new("time", DataType::Int64, false),
            Field::new("open", DataType::Float64, false),
            Field::new("high", DataType::Float64, false),
            Field::new("low", DataType::Float64, false),
            Field::new("close", DataType::Float64, false),
            Field::new("volume", DataType::Float64, false),
        ]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int64Array::from(vec![1_i64, 2])),
                Arc::new(Float64Array::from(vec![10.0, 11.0])),
                Arc::new(Float64Array::from(vec![12.0, 13.0])),
                Arc::new(Float64Array::from(vec![9.0, 10.0])),
                Arc::new(Float64Array::from(vec![11.0, 12.0])),
                Arc::new(Float64Array::from(vec![100.0, 200.0])),
            ],
        )
        .unwrap();

        let path = std::env::temp_dir().join("wkbt_parquet_round_trip.parquet");
        let file = std::fs::File::create(&path).unwrap();
        let mut writer = ArrowWriter::try_new(file, schema, None).unwrap();
        writer.write(&batch).unwrap();
        writer.close().unwrap();

        let candles = load_candles(&path).unwrap();
        assert_eq!(candles.len(), 2);
        assert_eq!(candles[0].time, 1);
        assert!((candles[0].open - 10.0).abs() < 1e-9);
        assert!((candles[1].close - 12.0).abs() < 1e-9);
        assert!((candles[1].volume - 200.0).abs() < 1e-9);

        std::fs::remove_file(&path).ok();
    }

    #[cfg(not(feature = "parquet"))]
    #[test]
    fn parquet_without_feature_errors() {
        assert!(load_parquet(Path::new("x.parquet")).is_err());
    }
}
