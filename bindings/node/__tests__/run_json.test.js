'use strict';

const { test } = require('node:test');
const assert = require('node:assert');
const { runJson } = require('..');

test('runJson runs a request bundle', () => {
  const request = JSON.stringify({
    capital: 1000,
    spec: {
      symbol: 'x', timeframe: '1h', indicators: {},
      entry: { gt: [{ price: 'close' }, 100] },
      exit: { lt: [{ price: 'close' }, 100] },
      sizing: { type: 'fixed_qty', qty: 1 },
    },
    candles: [
      { time: 0, open: 100, high: 101, low: 100, close: 101 },
      { time: 1, open: 102, high: 103, low: 102, close: 103 },
      { time: 2, open: 104, high: 104, low: 99, close: 99 },
      { time: 3, open: 98, high: 98, low: 97, close: 97 },
    ],
  });
  const report = JSON.parse(runJson(request));
  assert.strictEqual(report.metrics.num_trades, 1);
  assert.ok(Math.abs(report.trades[0].entry_price - 102) < 1e-9);
});
