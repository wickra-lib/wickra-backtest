"""Wickra Backtest — a streaming-native backtester built on Wickra indicators.

A strategy is **data** (a JSON spec / dict), so the same strategy runs
identically here and across every other Wickra language binding, and the
backtest values match a live run by construction.

Example::

    import wickra_backtest as wbt

    spec = {
        "symbol": "BTCUSDT", "timeframe": "1h",
        "indicators": {"fast": {"type": "Ema", "params": [12]},
                       "slow": {"type": "Ema", "params": [26]}},
        "entry": {"cross_above": ["fast", "slow"]},
        "exit":  {"cross_below": ["fast", "slow"]},
        "sizing": {"type": "fixed_fraction", "fraction": 0.95},
    }
    report = wbt.run(opens, highs, lows, closes, spec=spec)
    print(report["metrics"]["return_pct"], report["metrics"]["sharpe"])
"""

from __future__ import annotations

import json

from ._wickra_backtest import __version__, run as _run

__all__ = ["run", "__version__"]


def run(open, high, low, close, volume=None, time=None, *, spec, capital=10_000.0):
    """Run a backtest of ``spec`` over the given OHLCV arrays.

    ``open``/``high``/``low``/``close`` are required sequences (lists, NumPy
    arrays, ``array.array`` …). ``volume`` defaults to zeros and ``time`` to
    ``range(len)``. ``spec`` is a dict or a JSON string. Returns the report as a
    dict (``metrics``, ``trades``, ``equity``, …).
    """
    o = [float(x) for x in open]
    h = [float(x) for x in high]
    lo = [float(x) for x in low]
    c = [float(x) for x in close]
    n = len(o)
    v = [float(x) for x in volume] if volume is not None else [0.0] * n
    t = [int(x) for x in time] if time is not None else list(range(n))
    spec_json = spec if isinstance(spec, str) else json.dumps(spec)
    return json.loads(_run(o, h, lo, c, v, t, spec_json, float(capital)))
