"""Cross-library backtest throughput: wickra-backtest vs backtrader vs vectorbt.

Runs the *same* SMA-crossover strategy over the *same* synthetic candle series
through each library and reports end-to-end backtest throughput (bars/second).
Libraries that are not installed are skipped automatically, so the script always
produces output.

Usage::

    python -m benchmarks.compare_libraries
    python -m benchmarks.compare_libraries --size 100000 --repeat 3

Honest caveats:

- This measures *backtest throughput on identical data*, not identical results.
  Each library models fills, sizing and costs differently, so the trade counts
  and P&L will not match exactly; the point is the speed of the engine loop.
- wickra-backtest is an O(1)-per-bar streaming engine in Rust. backtrader is a
  pure-Python event loop. vectorbt is vectorised NumPy (fast, but it recomputes
  over the whole array and is not a streaming/live engine).
- Numbers vary by machine; run it yourself rather than trusting a quoted figure.
"""

from __future__ import annotations

import argparse
import math
import time
from typing import Callable, Optional

FAST, SLOW = 10, 30


def make_candles(n: int):
    """A deterministic sine-wave price path with OHLCV arrays."""
    close = [100.0 + 10.0 * math.sin(i / 20.0) + (i % 7) * 0.1 for i in range(n)]
    open_ = [close[i - 1] if i else close[0] for i in range(n)]
    high = [max(open_[i], close[i]) + 0.5 for i in range(n)]
    low = [min(open_[i], close[i]) - 0.5 for i in range(n)]
    volume = [1000.0] * n
    return open_, high, low, close, volume


def best_time(fn: Callable[[], object], repeat: int) -> float:
    """Run `fn` `repeat` times and return the fastest wall-clock seconds."""
    best = float("inf")
    for _ in range(repeat):
        start = time.perf_counter()
        fn()
        best = min(best, time.perf_counter() - start)
    return best


# --------------------------------------------------------------------------- #
# wickra-backtest
# --------------------------------------------------------------------------- #
def bench_wickra(candles, repeat: int) -> Optional[float]:
    try:
        import wickra_backtest as wbt
    except ImportError:
        return None
    open_, high, low, close, volume = candles
    spec = {
        "symbol": "x",
        "timeframe": "1d",
        "indicators": {
            "fast": {"type": "Sma", "params": [FAST]},
            "slow": {"type": "Sma", "params": [SLOW]},
        },
        "entry": {"cross_above": ["fast", "slow"]},
        "exit": {"cross_below": ["fast", "slow"]},
        "sizing": {"type": "fixed_fraction", "fraction": 0.95},
    }
    return best_time(lambda: wbt.run(open_, high, low, close, volume, spec=spec), repeat)


# --------------------------------------------------------------------------- #
# backtrader
# --------------------------------------------------------------------------- #
def bench_backtrader(candles, repeat: int) -> Optional[float]:
    try:
        import backtrader as bt
        import pandas as pd
    except ImportError:
        return None
    open_, high, low, close, volume = candles
    n = len(close)
    index = pd.date_range("2000-01-01", periods=n, freq="D")
    frame = pd.DataFrame(
        {"open": open_, "high": high, "low": low, "close": close, "volume": volume},
        index=index,
    )

    class SmaCross(bt.Strategy):
        def __init__(self):
            fast = bt.ind.SMA(period=FAST)
            slow = bt.ind.SMA(period=SLOW)
            self.crossover = bt.ind.CrossOver(fast, slow)

        def next(self):
            if not self.position:
                if self.crossover > 0:
                    self.buy()
            elif self.crossover < 0:
                self.close()

    def run() -> None:
        cerebro = bt.Cerebro(stdstats=False)
        cerebro.adddata(bt.feeds.PandasData(dataname=frame))
        cerebro.addstrategy(SmaCross)
        cerebro.broker.setcash(10_000.0)
        cerebro.run(runonce=False)

    return best_time(run, repeat)


# --------------------------------------------------------------------------- #
# vectorbt
# --------------------------------------------------------------------------- #
def bench_vectorbt(candles, repeat: int) -> Optional[float]:
    try:
        import numpy as np
        import vectorbt as vbt
    except ImportError:
        return None
    _, _, _, close, _ = candles
    price = np.asarray(close)

    def run() -> None:
        fast = vbt.MA.run(price, FAST).ma.to_numpy().flatten()
        slow = vbt.MA.run(price, SLOW).ma.to_numpy().flatten()
        entries = (fast[:-1] <= slow[:-1]) & (fast[1:] > slow[1:])
        exits = (fast[:-1] >= slow[:-1]) & (fast[1:] < slow[1:])
        entries = np.concatenate([[False], entries])
        exits = np.concatenate([[False], exits])
        vbt.Portfolio.from_signals(price, entries, exits, init_cash=10_000.0)

    return best_time(run, repeat)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--size", type=int, default=50_000, help="number of bars")
    parser.add_argument("--repeat", type=int, default=3, help="timed repetitions (fastest wins)")
    args = parser.parse_args()

    candles = make_candles(args.size)
    n = args.size

    benches = [
        ("wickra-backtest", bench_wickra),
        ("backtrader", bench_backtrader),
        ("vectorbt", bench_vectorbt),
    ]

    print(f"Backtest throughput over {n:,} bars (SMA {FAST}/{SLOW} cross), best of {args.repeat}:\n")
    print(f"  {'library':<18}{'seconds':>12}{'bars/sec':>16}")
    print(f"  {'-' * 16:<18}{'-' * 10:>12}{'-' * 14:>16}")
    baseline: Optional[float] = None
    for name, fn in benches:
        secs = fn(candles, args.repeat)
        if secs is None:
            print(f"  {name:<18}{'(not installed)':>28}")
            continue
        rate = n / secs if secs > 0 else float("inf")
        if name == "wickra-backtest":
            baseline = secs
        speed = f"{rate:,.0f}"
        print(f"  {name:<18}{secs:>12.4f}{speed:>16}")
    if baseline is not None:
        print("\n  Same data, same strategy intent; fill/cost models differ - this is")
        print("  engine-loop throughput, not a results-parity check.")


if __name__ == "__main__":
    main()
