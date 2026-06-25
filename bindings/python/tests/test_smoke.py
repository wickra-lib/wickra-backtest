"""Smoke tests for the wickra-backtest Python binding."""

import math

import wickra_backtest as wbt


PRICE_SPEC = {
    "symbol": "x",
    "timeframe": "1h",
    "indicators": {},
    "entry": {"gt": [{"price": "close"}, 100]},
    "exit": {"lt": [{"price": "close"}, 100]},
    "sizing": {"type": "fixed_qty", "qty": 1},
}


def test_version_present():
    assert isinstance(wbt.__version__, str) and wbt.__version__


def test_hand_computed_round_trip():
    # Mirrors the Rust engine's hand-computed test: one trade, pnl -4.
    opens = [100.0, 102.0, 104.0, 98.0]
    highs = [101.0, 103.0, 104.0, 98.0]
    lows = [100.0, 102.0, 99.0, 97.0]
    closes = [101.0, 103.0, 99.0, 97.0]
    report = wbt.run(opens, highs, lows, closes, spec=PRICE_SPEC, capital=1000.0)
    assert report["metrics"]["num_trades"] == 1
    trade = report["trades"][0]
    assert math.isclose(trade["entry_price"], 102.0)
    assert math.isclose(trade["exit_price"], 98.0)
    assert math.isclose(trade["pnl"], -4.0)
    assert math.isclose(report["equity"][-1]["equity"], 996.0)


def test_ema_cross_runs():
    spec = {
        "symbol": "x",
        "timeframe": "1h",
        "indicators": {
            "fast": {"type": "Ema", "params": [5]},
            "slow": {"type": "Ema", "params": [15]},
        },
        "entry": {"cross_above": ["fast", "slow"]},
        "exit": {"cross_below": ["fast", "slow"]},
        "sizing": {"type": "fixed_fraction", "fraction": 0.5},
    }
    n = 60
    closes = [100.0 + 10.0 * math.sin(i / 3.0) for i in range(n)]
    highs = [c + 1.0 for c in closes]
    lows = [c - 1.0 for c in closes]
    report = wbt.run(closes, highs, lows, closes, spec=spec)
    assert len(report["equity"]) == n
    assert report["schema_version"] == 1


def test_bad_spec_raises():
    import pytest

    with pytest.raises(ValueError):
        wbt.run([1.0], [1.0], [1.0], [1.0], spec={"not": "valid"})


def test_mismatched_lengths_raise():
    import pytest

    with pytest.raises(ValueError):
        wbt.run([1.0, 2.0], [1.0], [1.0], [1.0], spec=PRICE_SPEC)
