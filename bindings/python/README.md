<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514-7" alt="Wickra Backtest — backtest and live are byte-identical" width="100%"></a>
</p>

[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/ci.svg)](https://github.com/wickra-lib/wickra-backtest/actions/workflows/ci.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-backtest)
[![PyPI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/pypi.svg)](https://pypi.org/project/wickra-backtest/)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/license.svg)](https://github.com/wickra-lib/wickra-backtest#license)

# Wickra Backtest — Python

---

> **▶ Live demo:** run a strategy in your browser and watch the equity curve build bar by bar — **[backtest-live.wickra.org](https://backtest-live.wickra.org)** · zero backend, the same engine this repository ships, compiled to WebAssembly.

**Backtest and live — for Python. `pip install wickra-backtest` — prebuilt wheels for Linux, macOS and Windows, nothing to compile.**

Streaming-native backtester for the [Wickra](https://github.com/wickra-lib/wickra)
indicator library. A strategy is a JSON spec, so the backtest values match a live
run and every other language binding by construction.

## Install

```bash
pip install wickra-backtest
```

Pre-built wheels ship for Linux, macOS and Windows — there is nothing to
compile and no C library to track down.

## Quick start

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

The same strategy also runs one bar at a time, which is what makes a backtest and
a live loop the same code path — swap the array for a socket and nothing else
changes:

```python
with wbt.StreamingBacktest(spec=spec, capital=10_000) as bt:
    for bar in feed:
        bt.step(bar.open, bar.high, bar.low, bar.close, bar.volume, bar.time)
        print(bt.num_trades, bt.latest_equity())
    report = bt.finish()
```

Strategies that read a side feed pass it per bar with
`bt.step(..., feeds={"reference": other_close})`.

## Benchmark

`benchmarks/` reports this binding's throughput over the shared core. It measures
the call overhead of PyO3, not a cross-library ratio (the same Rust core runs
under every binding) — see the repository
[BENCHMARKS.md](https://github.com/wickra-lib/wickra-backtest/blob/main/BENCHMARKS.md) for the
numbers, the machine and how each harness is run.

## Documentation

The full guide, the spec reference and the API documentation live in the main
repository and the documentation site:

- **Repository:** <https://github.com/wickra-lib/wickra-backtest>
- **Docs** (guides, spec reference, cookbook): <https://backtest.wickra.org>
- **Runnable example:** [`examples/python/`](https://github.com/wickra-lib/wickra-backtest/tree/main/examples/python)

- **Repository:** <https://github.com/wickra-lib/wickra-backtest>
- **Strategy spec reference:** [STRATEGY_SPEC.md](https://github.com/wickra-lib/wickra-backtest/blob/main/docs/STRATEGY_SPEC.md)
- **Cookbook:** [COOKBOOK.md](https://github.com/wickra-lib/wickra-backtest/blob/main/docs/COOKBOOK.md)
- **Built on Wickra:** <https://github.com/wickra-lib/wickra> · <https://docs.wickra.org>

The same `StrategySpec` runs identically across Rust, Python, Node.js, WASM, C,
C++, C#, Go, Java and R — one engine kernel, byte-identical reports.

Wickra Backtest ships native bindings for Python, Node.js, WASM and Rust, plus a C ABI hub that any
C-capable language (C, C++, C#, Go, Java, R) links against — all forwarding to the
same data-driven, `unsafe`-forbidden Rust core.

## Security

Found a security issue? **Please don't open a public issue.** Report it privately
via the repository's *Security* tab (*"Report a vulnerability"*) or email
**support@wickra.org** with a subject line starting `[wickra security]`. Full
policy: <https://github.com/wickra-lib/wickra-backtest/blob/main/SECURITY.md>.

## Disclaimer

Not a trading system. Backtest results are deterministic transforms of the input
data — they are not financial advice and are not indicative of future
performance. Any use in a live trading context is at your own risk. The software
is provided **as is**, without warranty of any kind; see the license files for
the full terms.

## License

Licensed under either of [Apache-2.0](https://github.com/wickra-lib/wickra-backtest/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/wickra-lib/wickra-backtest/blob/main/LICENSE-MIT) at your option.
