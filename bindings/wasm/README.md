# Wickra Backtest — WASM

WebAssembly binding for the [wickra-backtest](../../README.md) engine, built with
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

## Test

```bash
wasm-pack build --target nodejs && node --test tests/golden.test.cjs
```
