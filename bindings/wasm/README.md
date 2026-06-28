<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514" alt="Wickra — streaming-first technical indicators" width="100%"></a>
</p>

[![Built on Wickra](https://img.shields.io/badge/built%20on-wickra-3b82f6)](https://github.com/wickra-lib/wickra)
[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/ci.svg)](https://github.com/wickra-lib/wickra-backtest/actions/workflows/ci.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-backtest)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/license.svg)](../../README.md#license)

# Wickra Backtest — WASM

---

WASM binding for the [wickra-backtest](../../README.md) engine, built with
wasm-bindgen. Run a backtest **in the browser** (or any WASM host) with the same
kernel and values as every other binding — the report is byte-identical.

## Requirements

- [`wasm-pack`](https://rustwasm.github.io/wasm-pack/)

## Build

```bash
cd bindings/wasm
wasm-pack build --target nodejs   # or --target web / bundler
```

This emits a `pkg/` directory with the `.wasm` module and JS glue exporting `run`.

## Usage

```js
const wasm = require('./pkg/wickra_backtest_wasm.js'); // nodejs target

const spec = JSON.stringify({
  symbol: 'x', timeframe: '1h', indicators: {},
  entry: { gt: [{ price: 'close' }, 100] },
  exit:  { lt: [{ price: 'close' }, 100] },
  sizing: { type: 'fixed_qty', qty: 1 },
});

const report = JSON.parse(wasm.run(
  Float64Array.from([100, 102, 104, 98]),  // open
  Float64Array.from([101, 103, 104, 98]),  // high
  Float64Array.from([100, 102,  99, 97]),  // low
  Float64Array.from([101, 103,  99, 97]),  // close
  Float64Array.from([0, 0, 0, 0]),         // volume
  Float64Array.from([0, 1, 2, 3]),         // time
  spec,
  10_000,
));
console.log(report.metrics);
```

`run(open, high, low, close, volume, time, specJson, capital)` returns the
`BacktestReport` JSON string. Inputs are `Float64Array`s of equal length; an
invalid spec throws a `JsError`.

For strategies that use microstructure feeds, `run_json(requestJson)` takes one
request bundle (candles + spec + optional order-book / trade / derivatives /
cross-section / reference feeds) and returns the same report JSON. See the
[microstructure guide](../../docs/MICROSTRUCTURE.md) for the feed shapes.

## Test

```bash
wasm-pack build --target nodejs && node --test tests/golden.test.cjs
```

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
