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

from ._wickra_backtest import (
    StreamingBacktest as _StreamingBacktest,
    __version__,
    run as _run,
    run_json as _run_json,
)

__all__ = ["run", "run_json", "StreamingBacktest", "__version__"]


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


def run_json(request):
    """Run a backtest from a single request bundle: ``candles``, ``spec``,
    optional ``capital`` and optional ``reference`` / ``derivs`` / ``books`` /
    ``trades`` / ``sections`` feeds. ``request`` is a dict or JSON string.
    Returns the report as a dict."""
    request_json = request if isinstance(request, str) else json.dumps(request)
    return json.loads(_run_json(request_json))


class StreamingBacktest:
    """A backtest driven one bar at a time.

    :func:`run` needs the whole series up front. This drives the same engine bar
    by bar, so a live loop and a backtest are the same code path: feed it from a
    socket instead of from an array and the numbers it reports were produced the
    way the backtest produced them.

    Example::

        bt = wbt.StreamingBacktest(spec=spec, capital=10_000)
        for bar in feed:
            bt.step(bar.open, bar.high, bar.low, bar.close, bar.volume, bar.time)
            print(bt.num_trades, bt.latest_equity())
        report = bt.finish()

    It is also a context manager, so a run abandoned by an exception still
    releases the engine::

        with wbt.StreamingBacktest(spec=spec) as bt:
            ...
    """

    __slots__ = ("_inner", "_bars")

    def __init__(self, *, spec, capital=10_000.0):
        spec_json = spec if isinstance(spec, str) else json.dumps(spec)
        self._inner = _StreamingBacktest(spec_json, float(capital))
        self._bars = 0

    def step(self, open, high, low, close, volume=0.0, time=None, *, feeds=None):
        """Advance by one bar.

        ``time`` defaults to the number of bars fed so far, matching
        :func:`run`'s default of ``range(len)``. ``feeds`` optionally carries
        this bar's ``reference`` / ``deriv`` / ``orderbook`` / ``trades`` /
        ``cross_section`` for strategies that read a side feed.
        """
        t = self._bars if time is None else int(time)
        if feeds is not None:
            self.step_json(
                {
                    "candle": {
                        "time": t,
                        "open": float(open),
                        "high": float(high),
                        "low": float(low),
                        "close": float(close),
                        "volume": float(volume),
                    },
                    "feeds": feeds,
                }
            )
            return
        self._inner.step(
            float(open), float(high), float(low), float(close), float(volume), t
        )
        self._bars += 1

    def step_json(self, step):
        """Advance by one bar given as a request document (dict or JSON string):
        ``{"candle": {...}, "feeds": {...}}``, where ``feeds`` is optional."""
        step_json = step if isinstance(step, str) else json.dumps(step)
        self._inner.step_json(step_json)
        self._bars += 1

    def equity(self):
        """The equity curve so far, as a list of dicts."""
        return json.loads(self._inner.equity_json())

    def latest_equity(self):
        """The most recent equity point, or ``None`` before the first bar."""
        return json.loads(self._inner.latest_equity_json())

    @property
    def num_trades(self):
        """The number of closed trades so far."""
        return self._inner.num_trades

    @property
    def is_finished(self):
        """Whether the run has been finished or closed."""
        return self._inner.is_finished

    def finish(self):
        """Close any open position and return the report dict. Ends the run."""
        return json.loads(self._inner.finish_json())

    def close(self):
        """Drop the run without producing a report. Idempotent."""
        self._inner.close()

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb):
        self.close()
        return False

    def __repr__(self):
        return repr(self._inner)
