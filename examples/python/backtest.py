"""Run the shared EMA-cross strategy from Python, both ways.

    python examples/python/backtest.py

Reads the same `examples/sample.csv` and `examples/ema-cross.json` every other
language example uses, runs the whole series at once, then feeds the same bars
one at a time and checks that the two agree. That equality is the point of the
library: a live loop is the streaming path with a socket in place of the file,
so a backtest is not a separate model of the strategy.

Requires the binding: `maturin develop` in bindings/python, or an installed
`wickra-backtest` wheel.
"""

import csv
import json
from pathlib import Path

import wickra_backtest as wbt

ROOT = Path(__file__).resolve().parents[1]
CAPITAL = 10_000.0


def load_bars(path):
    """The CSV columns are time,open,high,low,close,volume."""
    with path.open(newline="", encoding="utf-8") as handle:
        return [
            (
                int(row["time"]),
                float(row["open"]),
                float(row["high"]),
                float(row["low"]),
                float(row["close"]),
                float(row["volume"]),
            )
            for row in csv.DictReader(handle)
        ]


def main():
    spec = json.loads((ROOT / "ema-cross.json").read_text(encoding="utf-8"))
    bars = load_bars(ROOT / "sample.csv")

    batch = wbt.run(
        [b[1] for b in bars],
        [b[2] for b in bars],
        [b[3] for b in bars],
        [b[4] for b in bars],
        [b[5] for b in bars],
        [b[0] for b in bars],
        spec=spec,
        capital=CAPITAL,
    )

    # The same run, driven bar by bar. Replace the loop with reads from a socket
    # and this is a live strategy; nothing else about it changes.
    with wbt.StreamingBacktest(spec=spec, capital=CAPITAL) as live:
        for time, open_, high, low, close, volume in bars:
            live.step(open_, high, low, close, volume, time)
        streamed = live.finish()

    metrics = streamed["metrics"]
    print(f"bars            {len(bars)}")
    print(f"trades          {metrics['num_trades']}")
    print(f"pnl             {metrics['pnl']:.2f}")
    print(f"return %        {metrics['return_pct']:.2f}")
    print(f"max drawdown    {metrics['max_drawdown']:.4f}")
    print(f"final equity    {streamed['equity'][-1]['equity']:.2f}")

    if streamed != batch:
        raise SystemExit("streaming and batch disagree -- that should be impossible")
    print("\nstreaming reproduces the batch report exactly")


if __name__ == "__main__":
    main()
