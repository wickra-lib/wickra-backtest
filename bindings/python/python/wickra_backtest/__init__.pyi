"""Type stubs for the wickra-backtest Python binding."""

from typing import Any, Dict, List, Mapping, Optional, Sequence, Union

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

class StreamingBacktest:
    """A backtest driven one bar at a time: the same engine as :func:`run`."""

    def __init__(
        self,
        *,
        spec: Union[Mapping[str, Any], str],
        capital: float = ...,
    ) -> None: ...
    def step(
        self,
        open: float,
        high: float,
        low: float,
        close: float,
        volume: float = ...,
        time: Optional[int] = ...,
        *,
        feeds: Optional[Mapping[str, Any]] = ...,
    ) -> None:
        """Advance by one bar; ``time`` defaults to the bars fed so far."""
        ...

    def step_json(self, step: Union[Mapping[str, Any], str]) -> None:
        """Advance by one bar given as a ``{"candle": ..., "feeds": ...}`` document."""
        ...

    def equity(self) -> List[Dict[str, Any]]:
        """The equity curve so far."""
        ...

    def latest_equity(self) -> Optional[Dict[str, Any]]:
        """The most recent equity point, or ``None`` before the first bar."""
        ...

    @property
    def num_trades(self) -> int: ...
    @property
    def is_finished(self) -> bool: ...
    def finish(self) -> Dict[str, Any]:
        """Close any open position and return the report dict. Ends the run."""
        ...

    def close(self) -> None:
        """Drop the run without producing a report. Idempotent."""
        ...

    def __enter__(self) -> "StreamingBacktest": ...
    def __exit__(self, exc_type: Any, exc: Any, tb: Any) -> bool: ...
