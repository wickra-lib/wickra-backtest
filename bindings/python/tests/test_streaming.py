"""The streaming class must be the same engine as ``run``, one bar at a time."""

import math

import pytest

import wickra_backtest as wbt


PRICE_SPEC = {
    "symbol": "x",
    "timeframe": "1h",
    "indicators": {},
    "entry": {"gt": [{"price": "close"}, 100]},
    "exit": {"lt": [{"price": "close"}, 100]},
    "sizing": {"type": "fixed_qty", "qty": 1},
}

BARS = [
    # open, high, low, close
    (100.0, 101.0, 100.0, 101.0),
    (102.0, 103.0, 102.0, 103.0),
    (104.0, 104.0, 99.0, 99.0),
    (98.0, 98.0, 97.0, 97.0),
]


def _run_batch():
    return wbt.run(
        [b[0] for b in BARS],
        [b[1] for b in BARS],
        [b[2] for b in BARS],
        [b[3] for b in BARS],
        spec=PRICE_SPEC,
        capital=1000.0,
    )


def test_streaming_reproduces_the_batch_report():
    """The claim worth pinning: bar-by-bar and whole-series agree exactly."""
    bt = wbt.StreamingBacktest(spec=PRICE_SPEC, capital=1000.0)
    for open_, high, low, close in BARS:
        bt.step(open_, high, low, close)
    assert bt.finish() == _run_batch()


def test_step_json_matches_the_scalar_step():
    bt = wbt.StreamingBacktest(spec=PRICE_SPEC, capital=1000.0)
    for i, (open_, high, low, close) in enumerate(BARS):
        bt.step_json(
            {
                "candle": {
                    "time": i,
                    "open": open_,
                    "high": high,
                    "low": low,
                    "close": close,
                    "volume": 0.0,
                }
            }
        )
    assert bt.finish() == _run_batch()


def test_accessors_track_the_run():
    bt = wbt.StreamingBacktest(spec=PRICE_SPEC, capital=1000.0)
    assert bt.latest_equity() is None
    assert bt.equity() == []
    assert bt.num_trades == 0
    assert not bt.is_finished

    for open_, high, low, close in BARS[:3]:
        bt.step(open_, high, low, close)

    curve = bt.equity()
    assert len(curve) == 3
    assert bt.latest_equity() == curve[-1]
    # Bar 2 closed below 100, which is the exit *signal*; the fill lands on the
    # next bar's open, so nothing has closed yet.
    assert bt.num_trades == 0

    bt.step(*BARS[3])
    assert bt.num_trades == 1
    assert math.isclose(bt.equity()[-1]["equity"], 996.0)


def test_time_defaults_to_the_bar_index():
    bt = wbt.StreamingBacktest(spec=PRICE_SPEC, capital=1000.0)
    for open_, high, low, close in BARS:
        bt.step(open_, high, low, close)
    assert [point["time"] for point in bt.equity()] == [0, 1, 2, 3]


def test_a_finished_run_refuses_further_use():
    bt = wbt.StreamingBacktest(spec=PRICE_SPEC, capital=1000.0)
    bt.step(*BARS[0])
    bt.finish()
    assert bt.is_finished
    for call in (
        lambda: bt.step(*BARS[1]),
        lambda: bt.equity(),
        lambda: bt.latest_equity(),
        lambda: bt.finish(),
    ):
        with pytest.raises(ValueError, match="finished"):
            call()


def test_context_manager_releases_an_abandoned_run():
    with wbt.StreamingBacktest(spec=PRICE_SPEC, capital=1000.0) as bt:
        bt.step(*BARS[0])
        assert not bt.is_finished
    assert bt.is_finished
    bt.close()  # idempotent


def test_an_invalid_spec_raises():
    with pytest.raises(ValueError):
        wbt.StreamingBacktest(spec={"bad": True})


def test_feeds_reach_a_reference_reading_strategy():
    """A pairwise indicator is undefined without its reference series, so a spec
    that reads one proves the per-bar feed actually arrives -- and it must agree
    with the batch path fed the same reference.

    The series is a sine path rather than a geometric one: a constant-growth path
    has constant log returns, which drives the correlation's variance to zero and
    makes the indicator report nothing at all.
    """
    closes = [100 + 10 * math.sin(i * 0.5) for i in range(24)]
    bars = [(c, c + 1.0, c - 1.0, c) for c in closes]
    reference = [2 * c for c in closes]
    spec = {
        "symbol": "x",
        "timeframe": "1h",
        "indicators": {"corr": {"type": "PearsonCorrelation", "params": [5]}},
        "entry": {"gt": ["corr", 0.5]},
        "exit": {"lt": ["corr", -0.5]},
        "sizing": {"type": "fixed_qty", "qty": 1},
    }

    def candles(values):
        return [
            {
                "time": i,
                "open": v[0],
                "high": v[1],
                "low": v[2],
                "close": v[3],
                "volume": 0.0,
            }
            for i, v in enumerate(values)
        ]

    bt = wbt.StreamingBacktest(spec=spec, capital=1000.0)
    for i, (open_, high, low, close) in enumerate(bars):
        bt.step(open_, high, low, close, feeds={"reference": reference[i]})
    streamed = bt.finish()

    batch = wbt.run_json(
        {
            "spec": spec,
            "capital": 1000.0,
            "candles": candles(bars),
            "reference": candles([(r, r, r, r) for r in reference]),
        }
    )
    assert streamed == batch
    assert streamed["metrics"]["num_trades"] == 1

    # The feed is load-bearing: without it the correlation never resolves, so the
    # strategy never fires and the two runs cannot agree.
    blind = wbt.StreamingBacktest(spec=spec, capital=1000.0)
    for open_, high, low, close in bars:
        blind.step(open_, high, low, close)
    blind_report = blind.finish()
    assert blind_report["metrics"]["num_trades"] == 0
    assert blind_report != streamed
