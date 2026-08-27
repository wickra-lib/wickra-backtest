"""Streaming golden parity: driving each shared case one bar at a time must
reproduce the same canonical report (golden/expected/) the batch entry point
produces. The batch parity lives in test_golden.py; this pins that the streaming
reach did not drift away from it."""

import json
from pathlib import Path

import wickra_backtest

GOLDEN = Path(__file__).resolve().parents[3] / "golden"


def test_streaming_golden_parity():
    cases = sorted((GOLDEN / "cases").glob("*.json"))
    assert cases, "no golden cases found"
    for path in cases:
        case = json.loads(path.read_text(encoding="utf-8"))
        bt = wickra_backtest.StreamingBacktest(
            spec=case["spec"], capital=case["capital"]
        )
        for i in range(len(case["close"])):
            bt.step(
                case["open"][i],
                case["high"][i],
                case["low"][i],
                case["close"][i],
                case["volume"][i],
                case["time"][i],
            )
        want = json.loads(
            (GOLDEN / "expected" / f'{case["name"]}.json').read_text(encoding="utf-8")
        )
        assert bt.finish() == want, f'streaming mismatch for {case["name"]}'
