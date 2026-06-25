# Wickra Backtest — Python

Streaming-native backtester for the [Wickra](https://github.com/wickra-lib/wickra)
indicator library. A strategy is a JSON spec, so the backtest values match a live
run and every other language binding by construction.

```python
import wickra_backtest as wbt

spec = {
    "symbol": "BTCUSDT", "timeframe": "1h",
    "indicators": {"fast": {"type": "Ema", "params": [12]},
                   "slow": {"type": "Ema", "params": [26]}},
    "entry": {"cross_above": ["fast", "slow"]},
    "exit":  {"cross_below": ["fast", "slow"]},
    "sizing": {"type": "fixed_fraction", "fraction": 0.95},
}
report = wbt.run(opens, highs, lows, closes, spec=spec)
print(report["metrics"])
```

Lists, `array.array` and NumPy arrays all work as inputs (NumPy is not required).
