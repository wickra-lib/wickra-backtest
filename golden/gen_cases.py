#!/usr/bin/env python3
"""Generate the golden parity cases under golden/cases/.

Each case is a self-contained {name, capital, spec, open/high/low/close/volume/
time arrays} JSON file. The Rust engine turns each case into the expected
report (golden/expected/<name>.json, written by the `golden` integration test
in bless mode); every language binding then asserts its own output equals that
same expected report — byte-for-byte for the JSON-returning bindings.

Run from the repo root:
    python golden/gen_cases.py
"""
from __future__ import annotations

import json
import math
from pathlib import Path

OUT = Path(__file__).resolve().parent / "cases"


def candles(prices, start_time=0):
    o, h, l, c, v, t = [], [], [], [], [], []
    for i, px in enumerate(prices):
        o.append(round(px, 4))
        h.append(round(px + 0.5, 4))
        l.append(round(px - 0.5, 4))
        c.append(round(px, 4))
        v.append(1000.0)
        t.append(start_time + i)
    return {"open": o, "high": h, "low": l, "close": c, "volume": v, "time": t}


def case(name, capital, spec, prices):
    body = {"name": name, "capital": capital, "spec": spec}
    body.update(candles(prices))
    (OUT / f"{name}.json").write_text(
        json.dumps(body, indent=1) + "\n", encoding="utf-8"
    )
    print(f"  {name}: {len(prices)} bars")


def main():
    OUT.mkdir(parents=True, exist_ok=True)

    # 1. Price threshold, no indicators — the canonical hand-computed case.
    case(
        "price_threshold",
        1000.0,
        {
            "symbol": "x",
            "timeframe": "1h",
            "indicators": {},
            "entry": {"gt": [{"price": "close"}, 100]},
            "exit": {"lt": [{"price": "close"}, 100]},
            "sizing": {"type": "fixed_qty", "qty": 1},
        },
        [101.0, 103.0, 99.0, 97.0],
    )

    # 2. EMA crossover — two scalar indicators.
    ema_prices = [100 + 10 * math.sin(i * 0.3) + i * 0.1 for i in range(40)]
    case(
        "ema_cross",
        10000.0,
        {
            "symbol": "x",
            "timeframe": "1h",
            "indicators": {
                "fast": {"type": "Ema", "params": [3]},
                "slow": {"type": "Ema", "params": [8]},
            },
            "entry": {"cross_above": ["fast", "slow"]},
            "exit": {"cross_below": ["fast", "slow"]},
            "sizing": {"type": "fixed_qty", "qty": 1},
        },
        ema_prices,
    )

    # 3. RSI mean reversion — a scalar oscillator with thresholds.
    rsi_prices = [100 + 15 * math.sin(i * 0.5) for i in range(40)]
    case(
        "rsi_meanrev",
        10000.0,
        {
            "symbol": "x",
            "timeframe": "1h",
            "indicators": {"r": {"type": "Rsi", "params": [14]}},
            "entry": {"lt": ["r", 30]},
            "exit": {"gt": ["r", 70]},
            "sizing": {"type": "fixed_qty", "qty": 1},
        },
        rsi_prices,
    )

    # 4. MACD long/short — a multi-output indicator with "name.field" refs.
    macd_prices = [
        100 + 8 * math.sin(i * 0.2) + 5 * math.sin(i * 0.05) for i in range(80)
    ]
    case(
        "macd_long_short",
        10000.0,
        {
            "symbol": "x",
            "timeframe": "1h",
            "indicators": {"m": {"type": "MacdIndicator", "params": [12, 26, 9]}},
            "entry": {"cross_above": ["m.macd", "m.signal"]},
            "exit": {"cross_below": ["m.macd", "m.signal"]},
            "short_entry": {"cross_below": ["m.macd", "m.signal"]},
            "short_exit": {"cross_above": ["m.macd", "m.signal"]},
            "sizing": {"type": "fixed_qty", "qty": 1},
        },
        macd_prices,
    )


if __name__ == "__main__":
    main()
    print("golden cases written.")
