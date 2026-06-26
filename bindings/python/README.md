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

## Documentation

- **Repository:** <https://github.com/wickra-lib/wickra-backtest>
- **Strategy spec reference:** [STRATEGY_SPEC.md](../../docs/STRATEGY_SPEC.md)
- **Cookbook:** [COOKBOOK.md](../../docs/COOKBOOK.md)
- **Built on Wickra:** <https://github.com/wickra-lib/wickra> · <https://docs.wickra.org>

The same `StrategySpec` runs identically across Rust, Python, Node.js, WASM, C,
C++, C#, Go, Java and R — one engine kernel, byte-identical reports.

## Security

Found a security issue? **Please don't open a public issue.** Report it privately
via the repository's *Security* tab (*"Report a vulnerability"*) or email
**support@wickra.org**. Full policy:
<https://github.com/wickra-lib/wickra-backtest/blob/main/SECURITY.md>.

## Disclaimer

Not a trading system. Backtest results are deterministic transforms of the input
data — they are not financial advice and are not indicative of future
performance. Any use in a live trading context is at your own risk. Provided
**as is**, without warranty of any kind.

## License

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache-2.0](../../LICENSE-APACHE) at your option.
