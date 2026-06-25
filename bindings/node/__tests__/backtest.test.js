const { test } = require('node:test')
const assert = require('node:assert')
const { run, version } = require('../index.js')

const PRICE_SPEC = JSON.stringify({
  symbol: 'x',
  timeframe: '1h',
  indicators: {},
  entry: { gt: [{ price: 'close' }, 100] },
  exit: { lt: [{ price: 'close' }, 100] },
  sizing: { type: 'fixed_qty', qty: 1 },
})

test('version is a non-empty string', () => {
  assert.ok(typeof version() === 'string' && version().length > 0)
})

test('hand-computed round trip matches the Rust engine', () => {
  const open = [100, 102, 104, 98]
  const high = [101, 103, 104, 98]
  const low = [100, 102, 99, 97]
  const close = [101, 103, 99, 97]
  const volume = [0, 0, 0, 0]
  const time = [0, 1, 2, 3]
  const r = JSON.parse(run(open, high, low, close, volume, time, PRICE_SPEC, 1000))
  assert.strictEqual(r.metrics.num_trades, 1)
  assert.ok(Math.abs(r.trades[0].entry_price - 102) < 1e-9)
  assert.ok(Math.abs(r.trades[0].exit_price - 98) < 1e-9)
  assert.ok(Math.abs(r.trades[0].pnl - -4) < 1e-9)
  assert.ok(Math.abs(r.equity[r.equity.length - 1].equity - 996) < 1e-9)
})

test('ema crossover runs', () => {
  const spec = JSON.stringify({
    symbol: 'x',
    timeframe: '1h',
    indicators: { fast: { type: 'Ema', params: [5] }, slow: { type: 'Ema', params: [15] } },
    entry: { cross_above: ['fast', 'slow'] },
    exit: { cross_below: ['fast', 'slow'] },
    sizing: { type: 'fixed_fraction', fraction: 0.5 },
  })
  const n = 60
  const close = Array.from({ length: n }, (_, i) => 100 + 10 * Math.sin(i / 3))
  const high = close.map((c) => c + 1)
  const low = close.map((c) => c - 1)
  const volume = new Array(n).fill(0)
  const time = Array.from({ length: n }, (_, i) => i)
  const r = JSON.parse(run(close, high, low, close, volume, time, spec))
  assert.strictEqual(r.equity.length, n)
  assert.strictEqual(r.schema_version, 1)
})

test('invalid spec throws', () => {
  assert.throws(() => run([1], [1], [1], [1], [0], [0], JSON.stringify({ not: 'valid' })))
})
