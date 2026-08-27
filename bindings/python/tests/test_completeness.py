"""Parity guard: the Python binding exposes exactly the public surface of the
backtester (mirrors the completeness check in the main wickra repo).

The assertions are exact-set rather than "these names exist", because drift runs
both ways. A dropped export breaks callers; an export that only this binding has
is a portability lie -- the same strategy would stop being runnable everywhere,
which is the whole premise. Either direction fails here."""

import wickra_backtest as wbt

EXPORTS = {"run", "run_json", "StreamingBacktest", "__version__"}

# The streaming class is the binding's other half: a dropped method would leave
# `run` working and the "backtest and live are one code path" claim quietly
# false, which no value-comparing test would catch.
STREAM_METHODS = {"step", "step_json", "equity", "latest_equity", "finish", "close"}
STREAM_PROPERTIES = {"num_trades", "is_finished"}


def test_public_surface_is_exactly_the_declared_one():
    assert set(wbt.__all__) == EXPORTS
    for name in EXPORTS:
        assert hasattr(wbt, name), f"Python binding is missing {name}"
    assert callable(wbt.run)
    assert callable(wbt.run_json)
    assert isinstance(wbt.__version__, str)


def test_streaming_surface_is_exactly_the_declared_one():
    public = {name for name in vars(wbt.StreamingBacktest) if not name.startswith("_")}
    assert public == STREAM_METHODS | STREAM_PROPERTIES

    for method in STREAM_METHODS:
        assert callable(getattr(wbt.StreamingBacktest, method, None)), (
            f"StreamingBacktest is missing {method}"
        )
    for prop in STREAM_PROPERTIES:
        assert isinstance(getattr(wbt.StreamingBacktest, prop, None), property), (
            f"StreamingBacktest is missing the {prop} property"
        )


def test_streaming_is_a_context_manager():
    # Documented in the README and used by callers to release a run abandoned by
    # an exception, so it is part of the surface, not an implementation detail.
    assert callable(getattr(wbt.StreamingBacktest, "__enter__", None))
    assert callable(getattr(wbt.StreamingBacktest, "__exit__", None))
