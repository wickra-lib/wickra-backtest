# Examples

`ema-cross.json` is a fast/slow EMA crossover with a trailing stop;
`sample.csv` is a small synthetic OHLCV series.

Run the backtest with the `wkbt` CLI:

```bash
cargo run --bin wkbt -- run --data examples/sample.csv --spec examples/ema-cross.json
```

Write the full report plus the trade and equity streams:

```bash
cargo run --bin wkbt -- run \
  --data examples/sample.csv \
  --spec examples/ema-cross.json \
  --report report.json \
  --trades trades.jsonl \
  --equity equity.jsonl
```

The same strategy spec is just data, so it runs identically from every Wickra
language binding once those land — the backtest values match live, by construction.
