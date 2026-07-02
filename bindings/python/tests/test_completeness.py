"""Parity guard: the Python binding exposes the full public surface of the
backtester, so an export dropped in a refactor fails loudly here (mirrors the
completeness check in the main wickra repo)."""

import wickra_backtest as wbt

EXPORTS = ["run", "run_json", "__version__"]


def test_public_surface_complete():
    for name in EXPORTS:
        assert hasattr(wbt, name), f"Python binding is missing {name}"
    assert callable(wbt.run)
    assert callable(wbt.run_json)
    assert isinstance(wbt.__version__, str)
