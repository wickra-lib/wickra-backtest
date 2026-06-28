//! Throughput benchmark: how many bars/second the engine backtests.
//!
//! Run with `cargo bench -p wickra-backtest-bench`.

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::hint::black_box;
use wickra_backtest_core::{run, Candle, StrategySpec};

const EMA_CROSS: &str = r#"{
  "symbol": "BTCUSDT", "timeframe": "1h",
  "indicators": { "fast": {"type": "Ema", "params": [12]},
                  "slow": {"type": "Ema", "params": [26]} },
  "entry": {"cross_above": ["fast", "slow"]},
  "exit":  {"cross_below": ["fast", "slow"]},
  "sizing": {"type": "fixed_fraction", "fraction": 0.95},
  "costs": {"taker_bps": 5, "slippage": {"type": "fixed_bps", "bps": 2}},
  "risk": {"trailing_stop_pct": 5.0}
}"#;

/// A deterministic synthetic OHLCV series (trend + oscillation), so the strategy
/// actually trades.
fn make_candles(n: usize) -> Vec<Candle> {
    (0..n)
        .map(|i| {
            let x = i as f64;
            let osc = 12.0 * (x / 18.0).sin();
            let trend = (x / 200.0).sin() * 30.0;
            let close = 100.0 + trend + osc;
            let open = 100.0 + trend + 12.0 * ((x - 1.0) / 18.0).sin();
            Candle {
                time: i64::try_from(i).unwrap_or_default() * 3600,
                open,
                high: open.max(close) + 1.5,
                low: open.min(close) - 1.5,
                close,
                volume: 1000.0,
            }
        })
        .collect()
}

fn bench_backtest(c: &mut Criterion) {
    let spec = StrategySpec::parse(EMA_CROSS).unwrap();
    for &n in &[10_000usize, 100_000] {
        let candles = make_candles(n);
        let mut group = c.benchmark_group("ema_cross");
        group.throughput(Throughput::Elements(n as u64));
        group.bench_function(format!("{n}_bars"), |b| {
            b.iter(|| run(black_box(&spec), black_box(&candles)).unwrap());
        });
        group.finish();
    }
}

criterion_group!(benches, bench_backtest);
criterion_main!(benches);
