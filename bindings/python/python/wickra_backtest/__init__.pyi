"""Type stubs for the wickra-backtest Python binding."""

from typing import Any, Dict, Mapping, Optional, Sequence, Union

__version__: str

def run(
    open: Sequence[float],
    high: Sequence[float],
    low: Sequence[float],
    close: Sequence[float],
    volume: Optional[Sequence[float]] = ...,
    time: Optional[Sequence[int]] = ...,
    *,
    spec: Union[Mapping[str, Any], str],
    capital: float = ...,
) -> Dict[str, Any]:
    """Run a backtest of ``spec`` over the OHLCV arrays and return the report dict."""
    ...

def run_json(request: Union[Mapping[str, Any], str]) -> Dict[str, Any]:
    """Run a backtest from a single request bundle (dict or JSON string)."""
    ...
