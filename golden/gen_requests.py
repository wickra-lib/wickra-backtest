#!/usr/bin/env python3
"""Generate the feed golden requests under golden/requests/.

Each request is a self-contained `RunRequest` bundle (spec + candles + capital +
exactly one optional per-bar feed) that exercises a microstructure feed path:
derivatives, order book, trades, cross-section and the pairwise reference
series. The Rust engine turns each request into the expected report
(golden/expected_json/<name>.json, written by the `golden_json` integration test
in bless mode) through the unified `run_json` entry point; every language
binding then calls its own `run_json` wrapper on the same request and asserts the
result equals that expected report — byte-for-byte. This pins the feed paths
across all ten languages, not just the plain OHLCV path.

Run from the repo root:
    python golden/gen_requests.py
"""
from __future__ import annotations

import json
from pathlib import Path

OUT = Path(__file__).resolve().parent / "requests"


def flat_candles(n, price=100.0):
    return [
        {"time": i, "open": price, "high": price, "low": price, "close": price, "volume": 0.0}
        for i in range(n)
    ]


def deriv_tick(funding_rate):
    return {
        "funding_rate": funding_rate,
        "mark_price": 100.0,
        "index_price": 100.0,
        "futures_price": 100.0,
        "open_interest": 1000.0,
        "long_size": 600.0,
        "short_size": 400.0,
        "taker_buy_volume": 50.0,
        "taker_sell_volume": 40.0,
        "long_liquidation": 0.0,
        "short_liquidation": 0.0,
    }


def write(name, request):
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / f"{name}.json"
    path.write_text(json.dumps(request, indent=2) + "\n", encoding="utf-8")
    print(f"wrote {path.relative_to(Path(__file__).resolve().parent.parent)}")


def main():
    # 1. Derivatives feed: FundingRate passes the tick's funding rate through.
    write(
        "funding_long",
        {
            "capital": 10000.0,
            "spec": {
                "symbol": "x", "timeframe": "1h",
                "indicators": {"f": {"type": "FundingRate", "params": []}},
                "entry": {"gt": ["f", 0.0]},
                "exit": {"lt": ["f", -1.0]},
                "sizing": {"type": "fixed_qty", "qty": 1},
            },
            "candles": flat_candles(5),
            "derivs": [deriv_tick(0.01) for _ in range(5)],
        },
    )

    # 2. Order-book feed: a bid-heavy book gives a positive top-of-book imbalance.
    book = {
        "bids": [{"price": 100.0, "size": 9.0}],
        "asks": [{"price": 101.0, "size": 1.0}],
    }
    write(
        "orderbook_imbalance",
        {
            "capital": 10000.0,
            "spec": {
                "symbol": "x", "timeframe": "1h",
                "indicators": {"i": {"type": "OrderBookImbalanceTop1", "params": []}},
                "entry": {"gt": ["i", 0.0]},
                "exit": {"lt": ["i", -2.0]},
                "sizing": {"type": "fixed_qty", "qty": 1},
            },
            "candles": flat_candles(5),
            "books": [dict(book) for _ in range(5)],
        },
    )

    # 3. Trade feed: two buy trades per bar push cumulative volume delta positive.
    buy = {"price": 100.0, "size": 5.0, "side": "buy", "timestamp": 0}
    write(
        "trade_cvd",
        {
            "capital": 10000.0,
            "spec": {
                "symbol": "x", "timeframe": "1h",
                "indicators": {"cvd": {"type": "CumulativeVolumeDelta", "params": []}},
                "entry": {"gt": ["cvd", 0.0]},
                "exit": {"lt": ["cvd", -1.0]},
                "sizing": {"type": "fixed_qty", "qty": 1},
            },
            "candles": flat_candles(5),
            "trades": [[dict(buy), dict(buy)] for _ in range(5)],
        },
    )

    # 4. Cross-section feed: three advancers vs one decliner is breadth-positive.
    advancer = {"change": 1.0, "volume": 100.0, "new_high": False, "new_low": False}
    decliner = {"change": -1.0, "volume": 100.0, "new_high": False, "new_low": False}
    section = {"members": [advancer, advancer, advancer, decliner], "timestamp": 0}
    write(
        "cross_section_breadth",
        {
            "capital": 10000.0,
            "spec": {
                "symbol": "x", "timeframe": "1h",
                "indicators": {"ad": {"type": "AdvanceDecline", "params": []}},
                "entry": {"gt": ["ad", 0.0]},
                "exit": {"lt": ["ad", -100.0]},
                "sizing": {"type": "fixed_qty", "qty": 1},
            },
            "candles": flat_candles(4),
            "sections": [dict(section) for _ in range(4)],
        },
    )

    # 5. Reference series: a correlated reference drives a pairwise correlation.
    primary = [100.0, 101.0, 102.0, 101.0, 103.0, 102.0, 104.0, 103.0]
    reference = [50.0, 50.5, 51.0, 50.5, 51.5, 51.0, 52.0, 51.5]

    def candle_series(prices):
        return [
            {"time": i, "open": p, "high": p + 0.5, "low": p - 0.5, "close": p, "volume": 0.0}
            for i, p in enumerate(prices)
        ]

    write(
        "pair_correlation",
        {
            "capital": 10000.0,
            "spec": {
                "symbol": "x", "timeframe": "1h",
                "indicators": {"c": {"type": "PearsonCorrelation", "params": [3]}},
                "entry": {"gt": ["c", 0.5]},
                "exit": {"lt": ["c", -2.0]},
                "sizing": {"type": "fixed_qty", "qty": 1},
            },
            "candles": candle_series(primary),
            "reference": candle_series(reference),
        },
    )


if __name__ == "__main__":
    main()
