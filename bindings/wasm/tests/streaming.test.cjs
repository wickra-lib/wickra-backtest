'use strict';

// The WASM binding exposes a stateful StreamingBacktest handle: build from a
// spec, feed candles one at a time, read the equity tail as you go, then finish.
// Same engine and the same values as the batch `run_json`. Build first with:
//   wasm-pack build --target nodejs --out-dir pkg
// then run:  node --test tests/streaming.test.cjs
const test = require('node:test');
const assert = require('node:assert');
const wasm = require('../pkg/wickra_backtest_wasm.js');

const SPEC = {
  symbol: 'x', timeframe: '1h', indicators: {},
  entry: { gt: [{ price: 'close' }, 100] },
  exit: { lt: [{ price: 'close' }, 100] },
  sizing: { type: 'fixed_qty', qty: 1 },
};
const CANDLES = [
  { time: 0, open: 100, high: 101, low: 100, close: 101 },
  { time: 1, open: 102, high: 103, low: 102, close: 103 },
  { time: 2, open: 104, high: 104, low: 99, close: 99 },
  { time: 3, open: 98, high: 98, low: 97, close: 97 },
];

test('wasm StreamingBacktest steps bar-by-bar and matches the batch path', () => {
  const bt = new wasm.StreamingBacktest(JSON.stringify(SPEC), 1000);
  for (const candle of CANDLES) {
    bt.step(JSON.stringify(candle));
  }
  assert.ok(bt.numTrades() >= 1, 'expected at least one trade');
  const equity = JSON.parse(bt.equity());
  assert.strictEqual(equity.length, CANDLES.length);

  const report = JSON.parse(bt.finish());
  // The same engine as the batch run_json (see run_json.test.cjs).
  assert.strictEqual(report.metrics.num_trades, 1);
});

test('wasm StreamingBacktest is consumed after finish', () => {
  const bt = new wasm.StreamingBacktest(JSON.stringify(SPEC), 1000);
  bt.step(JSON.stringify(CANDLES[0]));
  bt.finish();
  assert.throws(() => bt.step(JSON.stringify(CANDLES[0])));
});
