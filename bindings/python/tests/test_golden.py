"""Golden parity: the Python binding asserts its output against the shared
golden reports (golden/expected/), pinning cross-language equality. The binding
returns the report as a dict, so the comparison is value-for-value against the
parsed expected JSON."""

import json
from pathlib import Path

import wickra_backtest

GOLDEN = Path(__file__).resolve().parents[3] / "golden"


def test_golden_parity():
    cases = sorted((GOLDEN / "cases").glob("*.json"))
    assert cases, "no golden cases found"
    for path in cases:
        case = json.loads(path.read_text(encoding="utf-8"))
        report = wickra_backtest.run(
            case["open"],
            case["high"],
            case["low"],
            case["close"],
            case["volume"],
            case["time"],
            spec=case["spec"],
            capital=case["capital"],
        )
        want = json.loads(
            (GOLDEN / "expected" / f'{case["name"]}.json').read_text(encoding="utf-8")
        )
        assert report == want, f'golden mismatch for {case["name"]}'
