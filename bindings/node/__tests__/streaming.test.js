'use strict'

// The streaming class must be the same engine as `run`, one bar at a time.

const { test } = require('node:test')
const assert = require('node:assert')
const { run, runJson, StreamingBacktest } = require('../index.js')

const PRICE_SPEC = JSON.stringify({
  symbol: 'x',
  timeframe: '1h',
  indicators: {},
  entry: { gt: [{ price: 'close' }, 100] },
  exit: { lt: [{ price: 'close' }, 100] },
  sizing: { type: 'fixed_qty', qty: 1 },
})

// open, high, low, close
const BARS = [
  [100, 101, 100, 101],
  [102, 103, 102, 103],
  [104, 104, 99, 99],
  [98, 98, 97, 97],
]

function batchReport() {
  return JSON.parse(
    run(
      BARS.map((b) => b[0]),
      BARS.map((b) => b[1]),
      BARS.map((b) => b[2]),
      BARS.map((b) => b[3]),
      BARS.map(() => 0),
      BARS.map((_, i) => i),
      PRICE_SPEC,
      1000,
    ),
  )
}

test('streaming reproduces the batch report', () => {
  const bt = new StreamingBacktest(PRICE_SPEC, 1000)
  for (const [o, h, l, c] of BARS) bt.step(o, h, l, c)
  assert.deepStrictEqual(JSON.parse(bt.finishJson()), batchReport())
})

test('stepJson matches the scalar step', () => {
  const bt = new StreamingBacktest(PRICE_SPEC, 1000)
  BARS.forEach(([open, high, low, close], time) => {
    bt.stepJson(
      JSON.stringify({ candle: { time, open, high, low, close, volume: 0 } }),
    )
  })
  assert.deepStrictEqual(JSON.parse(bt.finishJson()), batchReport())
})

test('accessors track the run', () => {
  const bt = new StreamingBacktest(PRICE_SPEC, 1000)
  assert.strictEqual(JSON.parse(bt.latestEquityJson()), null)
  assert.deepStrictEqual(JSON.parse(bt.equityJson()), [])
  assert.strictEqual(bt.numTrades, 0)
  assert.strictEqual(bt.isFinished, false)

  for (const [o, h, l, c] of BARS.slice(0, 3)) bt.step(o, h, l, c)

  const curve = JSON.parse(bt.equityJson())
  assert.strictEqual(curve.length, 3)
  assert.deepStrictEqual(JSON.parse(bt.latestEquityJson()), curve[2])
  // Bar 2 closed below 100, which is the exit *signal*; the fill lands on the
  // next bar's open, so nothing has closed yet.
  assert.strictEqual(bt.numTrades, 0)

  bt.step(...BARS[3])
  assert.strictEqual(bt.numTrades, 1)
})

test('time defaults to the bar index', () => {
  const bt = new StreamingBacktest(PRICE_SPEC, 1000)
  for (const [o, h, l, c] of BARS) bt.step(o, h, l, c)
  assert.deepStrictEqual(
    JSON.parse(bt.equityJson()).map((p) => p.time),
    [0, 1, 2, 3],
  )
})

test('capital defaults when omitted', () => {
  const bt = new StreamingBacktest(PRICE_SPEC)
  bt.step(...BARS[0])
  assert.strictEqual(JSON.parse(bt.equityJson())[0].equity, 10000)
})

test('a finished run refuses further use', () => {
  const bt = new StreamingBacktest(PRICE_SPEC, 1000)
  bt.step(...BARS[0])
  bt.finishJson()
  assert.strictEqual(bt.isFinished, true)
  assert.throws(() => bt.step(...BARS[1]), /finished/)
  assert.throws(() => bt.equityJson(), /finished/)
  assert.throws(() => bt.latestEquityJson(), /finished/)
  assert.throws(() => bt.finishJson(), /finished/)
})

test('close is idempotent and ends the run', () => {
  const bt = new StreamingBacktest(PRICE_SPEC, 1000)
  bt.step(...BARS[0])
  bt.close()
  bt.close()
  assert.strictEqual(bt.isFinished, true)
})

test('an invalid spec throws', () => {
  assert.throws(() => new StreamingBacktest(JSON.stringify({ bad: true })))
})

test('per-bar feeds reach a reference-reading strategy', () => {
  // A sine path, not a geometric one: constant growth means constant log
  // returns, which drives the correlation's variance to zero.
  const closes = Array.from({ length: 24 }, (_, i) => 100 + 10 * Math.sin(i * 0.5))
  const bars = closes.map((c) => [c, c + 1, c - 1, c])
  const reference = closes.map((c) => 2 * c)
  const spec = {
    symbol: 'x',
    timeframe: '1h',
    indicators: { corr: { type: 'PearsonCorrelation', params: [5] } },
    entry: { gt: ['corr', 0.5] },
    exit: { lt: ['corr', -0.5] },
    sizing: { type: 'fixed_qty', qty: 1 },
  }
  const candles = (rows) =>
    rows.map(([open, high, low, close], time) => ({
      time,
      open,
      high,
      low,
      close,
      volume: 0,
    }))

  const bt = new StreamingBacktest(JSON.stringify(spec), 1000)
  bars.forEach(([open, high, low, close], i) => {
    bt.stepJson(
      JSON.stringify({
        candle: { time: i, open, high, low, close, volume: 0 },
        feeds: { reference: reference[i] },
      }),
    )
  })
  const streamed = JSON.parse(bt.finishJson())

  const batch = JSON.parse(
    runJson(
      JSON.stringify({
        spec,
        capital: 1000,
        candles: candles(bars),
        reference: candles(reference.map((r) => [r, r, r, r])),
      }),
    ),
  )
  assert.deepStrictEqual(streamed, batch)
  assert.strictEqual(streamed.metrics.num_trades, 1)

  // The feed is load-bearing: without it the correlation never resolves.
  const blind = new StreamingBacktest(JSON.stringify(spec), 1000)
  for (const [o, h, l, c] of bars) blind.step(o, h, l, c)
  const blindReport = JSON.parse(blind.finishJson())
  assert.strictEqual(blindReport.metrics.num_trades, 0)
  assert.notDeepStrictEqual(blindReport, streamed)
})
