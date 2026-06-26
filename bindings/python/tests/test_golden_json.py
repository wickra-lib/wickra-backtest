"""Feed golden parity: each request bundle (golden/requests/) exercises a
microstructure feed path through the unified run_json entry point, and the
Python binding asserts its output against the shared expected reports
(golden/expected_json/), value-for-value against the parsed expected JSON. This
pins cross-language feed equality, not just the plain OHLCV path."""

import json
from pathlib import Path

import wickra_backtest

GOLDEN = Path(__file__).resolve().parents[3] / "golden"


def test_feed_golden_parity():
    requests = sorted((GOLDEN / "requests").glob("*.json"))
    assert requests, "no golden requests found"
    for path in requests:
        request = path.read_text(encoding="utf-8")
        report = wickra_backtest.run_json(request)
        want = json.loads(
            (GOLDEN / "expected_json" / f"{path.stem}.json").read_text(encoding="utf-8")
        )
        assert report == want, f"feed golden mismatch for {path.stem}"
