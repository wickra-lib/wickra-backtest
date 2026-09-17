<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514" alt="Wickra Backtest — backtest and live are byte-identical" width="100%"></a>
</p>

[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/ci.svg)](https://github.com/wickra-lib/wickra-backtest/actions/workflows/ci.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-backtest)
[![npm](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/npm.svg)](https://www.npmjs.com/package/wickra-backtest-wasm)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/license.svg)](https://github.com/wickra-lib/wickra-backtest#license)

# Wickra Backtest — WASM

---

> **▶ Live demo:** run a strategy in your browser and watch the equity curve build bar by bar — **[backtest-live.wickra.org](https://backtest-live.wickra.org)** · zero backend, the same engine this repository ships, compiled to WebAssembly.

**Backtest and live — for WASM. `npm install wickra-backtest-wasm` — pure WebAssembly, runs anywhere a modern JS engine does.**

WASM binding for the [wickra-backtest](https://github.com/wickra-lib/wickra-backtest) engine, built with
wasm-bindgen. Run a backtest **in the browser** (or any WASM host) with the same
kernel and values as every other binding — the report is byte-identical.

## Install

```bash
npm install wickra-backtest-wasm
```

### Requirements

- [`wasm-pack`](https://rustwasm.github.io/docs/wasm-pack/)

### Building from this repository (contributors)

**Build.**

```bash
cd bindings/wasm
wasm-pack build --target nodejs   # or --target web / bundler
```

This emits a `pkg/` directory with the `.wasm` module and JS glue exporting `run`.

**Test.**

```bash
wasm-pack build --target nodejs && node --test tests/golden.test.cjs
```

## Quick start

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
[microstructure guide](https://github.com/wickra-lib/wickra-backtest/blob/main/docs/MICROSTRUCTURE.md) for the feed shapes.

## Benchmark

`benchmarks/` reports this binding's throughput over the shared core. It measures
the call overhead of wasm-bindgen, not a cross-library ratio (the same Rust core runs
under every binding) — see the repository
[BENCHMARKS.md](https://github.com/wickra-lib/wickra-backtest/blob/main/BENCHMARKS.md) for the
numbers, the machine and how each harness is run.

## Documentation

The full guide, the spec reference and the API documentation live in the main
repository and the documentation site:

- **Repository:** <https://github.com/wickra-lib/wickra-backtest>
- **Docs** (guides, spec reference, cookbook): <https://backtest.wickra.org>
- **Runnable example:** [`examples/wasm/`](https://github.com/wickra-lib/wickra-backtest/tree/main/examples/wasm)

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
