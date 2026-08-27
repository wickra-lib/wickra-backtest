"""Parity guard: the Python binding exposes the full public surface of the
backtester, so an export dropped in a refactor fails loudly here (mirrors the
completeness check in the main wickra repo)."""

import wickra_backtest as wbt

EXPORTS = ["run", "run_json", "StreamingBacktest", "__version__"]


def test_public_surface_complete():
    for name in EXPORTS:
        assert hasattr(wbt, name), f"Python binding is missing {name}"
    assert callable(wbt.run)
    assert callable(wbt.run_json)
    assert isinstance(wbt.__version__, str)

    # The streaming class is the binding's other half: a dropped method would
    # leave `run` working and the "backtest and live are one code path" claim
    # quietly false, which no value-comparing test would catch.
    for method in (
        "step",
        "step_json",
        "equity",
        "latest_equity",
        "finish",
        "close",
    ):
        assert callable(getattr(wbt.StreamingBacktest, method, None)), (
            f"StreamingBacktest is missing {method}"
        )
    for prop in ("num_trades", "is_finished"):
        assert isinstance(getattr(wbt.StreamingBacktest, prop, None), property), (
            f"StreamingBacktest is missing the {prop} property"
        )
