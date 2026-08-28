"""Throughput benchmark for the wickra-backtest Python binding.

Measures what crossing the boundary costs. Every reach in this repository runs
the same Rust engine, so a difference between two bindings is not a difference
in the backtester -- it is the price of that language's FFI, paid once per bar
on the streaming path and once per run on the batch path. That is the number
worth knowing before choosing where to drive a live loop from.

The strategy is examples/ema-cross.json: two EMAs, a crossover, fractional
sizing, taker costs, slippage and a trailing stop. A realistic bar rather than
an empty one, so the figure includes the engine work a real strategy does.

Run after installing the binding (``maturin develop --release``)::

    python -m benchmarks.throughput                 # 200k bars (default)
    python -m benchmarks.throughput --bars 1000000
"""

from __future__ import annotations

import argparse
import json
import math
import time

import wickra_backtest as wbt

SPEC = json.dumps(
    {
        "symbol": "BTCUSDT",
        "timeframe": "1h",
        "indicators": {
            "ema_fast": {"type": "Ema", "params": [5]},
            "ema_slow": {"type": "Ema", "params": [15]},
        },
        "entry": {"cross_above": ["ema_fast", "ema_slow"]},
        "exit": {"cross_below": ["ema_fast", "ema_slow"]},
        "sizing": {"type": "fixed_fraction", "fraction": 0.95},
        "costs": {"taker_bps": 5, "slippage": {"type": "fixed_bps", "bps": 2}},
        "risk": {"trailing_stop_pct": 5.0},
    }
)
CAPITAL = 10_000.0


def make_series(bars: int):
    """Deterministic synthetic OHLCV.

    No RNG, so two runs are comparable and so are two languages: every binding's
    harness builds the series from this same formula.
    """
    open_, high, low, close, volume, time_ = [], [], [], [], [], []
    for i in range(bars):
        mid = 100.0 + math.sin(i * 0.001) * 20.0 + i * 1e-4
        c = mid + math.sin(i * 0.05) * 2.0
        o = close[i - 1] if i else c
        close.append(c)
        open_.append(o)
        high.append(max(o, c) + 1.5)
        low.append(min(o, c) - 1.5)
        volume.append(1000.0 + (i % 97) * 13)
        time_.append(i)
    return open_, high, low, close, volume, time_


def median_seconds(fn, reps: int = 3) -> float:
    """Median wall-clock seconds over `reps` runs, after one warmup."""
    fn()
    samples = []
    for _ in range(reps):
        start = time.perf_counter()
        fn()
        samples.append(time.perf_counter() - start)
    samples.sort()
    return samples[len(samples) // 2]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bars", type=int, default=200_000, help="bars per run")
    args = parser.parse_args()
    if args.bars < 1000:
        raise SystemExit("--bars must be at least 1000")

    open_, high, low, close, volume, time_ = make_series(args.bars)

    def streaming() -> None:
        with wbt.StreamingBacktest(spec=SPEC, capital=CAPITAL) as live:
            for i in range(args.bars):
                live.step(open_[i], high[i], low[i], close[i], volume[i], time_[i])
            live.finish()

    def batch() -> None:
        wbt.run(open_, high, low, close, volume, time_, spec=SPEC, capital=CAPITAL)

    streaming_s = median_seconds(streaming)
    batch_s = median_seconds(batch)

    print(f"wickra-backtest Python throughput — {args.bars:,} bars (median of 3 runs)\n")
    print(f"{'path':<14}{'bars/sec':>16}{'ns/bar':>12}")
    print("-" * 42)
    for label, seconds in (("streaming", streaming_s), ("batch", batch_s)):
        print(f"{label:<14}{args.bars / seconds:>16,.0f}{seconds / args.bars * 1e9:>12,.0f}")
    print(
        "\nStreaming crosses the boundary once per bar, with scalars. Batch crosses it"
        "\nonce per run, but marshals six full lists to do it -- which of the two wins"
        "\nis a property of the language, not of the engine behind both of them."
        "\nMachine-dependent — compare bindings on one machine, not across machines."
    )


if __name__ == "__main__":
    main()
