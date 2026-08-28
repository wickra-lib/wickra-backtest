// Throughput benchmark for the wickra-backtest Node binding.
//
// Measures what crossing the boundary costs. Every reach in this repository
// runs the same Rust engine, so a difference between two bindings is not a
// difference in the backtester -- it is the price of that language's FFI, paid
// once per bar on the streaming path and once per run on the batch path. That
// is the number worth knowing before choosing where to drive a live loop from.
//
// The strategy is examples/ema-cross.json: two EMAs, a crossover, fractional
// sizing, taker costs, slippage and a trailing stop. A realistic bar rather
// than an empty one, so the figure includes the engine work a real strategy
// actually does.
//
// Run after building the binding:
//
//   cd bindings/node && npm install && npx napi build --platform --release
//   node benchmarks/throughput.js               # 200k bars (default)
//   node benchmarks/throughput.js --bars 1000000

const { run, StreamingBacktest } = require('..');

const SPEC = JSON.stringify({
  symbol: 'BTCUSDT',
  timeframe: '1h',
  indicators: {
    ema_fast: { type: 'Ema', params: [5] },
    ema_slow: { type: 'Ema', params: [15] },
  },
  entry: { cross_above: ['ema_fast', 'ema_slow'] },
  exit: { cross_below: ['ema_fast', 'ema_slow'] },
  sizing: { type: 'fixed_fraction', fraction: 0.95 },
  costs: { taker_bps: 5, slippage: { type: 'fixed_bps', bps: 2 } },
  risk: { trailing_stop_pct: 5.0 },
});
const CAPITAL = 10000;

function parseBars() {
  const idx = process.argv.indexOf('--bars');
  if (idx !== -1 && process.argv[idx + 1]) {
    const n = Number(process.argv[idx + 1]);
    if (Number.isFinite(n) && n >= 1000) return Math.floor(n);
    console.error('--bars must be a number >= 1000');
    process.exit(1);
  }
  return 200_000;
}

const BARS = parseBars();

// Deterministic synthetic OHLCV -- no RNG, so two runs are comparable and so
// are two languages. The same formula is used by every binding's harness.
const open = new Array(BARS);
const high = new Array(BARS);
const low = new Array(BARS);
const close = new Array(BARS);
const volume = new Array(BARS);
const time = new Array(BARS);
for (let i = 0; i < BARS; i++) {
  const mid = 100 + Math.sin(i * 0.001) * 20 + i * 1e-4;
  close[i] = mid + Math.sin(i * 0.05) * 2;
  open[i] = i ? close[i - 1] : close[i];
  high[i] = Math.max(open[i], close[i]) + 1.5;
  low[i] = Math.min(open[i], close[i]) - 1.5;
  volume[i] = 1000 + (i % 97) * 13;
  time[i] = i;
}

// Median elapsed nanoseconds over a few repetitions, after one warmup pass.
function timeNs(fn, reps = 3) {
  fn();
  const samples = [];
  for (let r = 0; r < reps; r++) {
    const t0 = process.hrtime.bigint();
    fn();
    samples.push(Number(process.hrtime.bigint() - t0));
  }
  samples.sort((a, b) => a - b);
  return samples[Math.floor(samples.length / 2)];
}

const barsPerSecond = (ns) => BARS / (ns / 1e9);

const streamingNs = timeNs(() => {
  const bt = new StreamingBacktest(SPEC, CAPITAL);
  for (let i = 0; i < BARS; i++) {
    bt.step(open[i], high[i], low[i], close[i], volume[i], time[i]);
  }
  bt.finishJson();
});

const batchNs = timeNs(() => {
  run(open, high, low, close, volume, time, SPEC, CAPITAL);
});

const fmt = (n) => Math.round(n).toLocaleString('en-US');
console.log(`wickra-backtest Node throughput — ${fmt(BARS)} bars (median of 3 runs)\n`);
console.log(`${'path'.padEnd(14)}${'bars/sec'.padStart(16)}${'ns/bar'.padStart(12)}`);
console.log('-'.repeat(42));
console.log(`${'streaming'.padEnd(14)}${fmt(barsPerSecond(streamingNs)).padStart(16)}${fmt(streamingNs / BARS).padStart(12)}`);
console.log(`${'batch'.padEnd(14)}${fmt(barsPerSecond(batchNs)).padStart(16)}${fmt(batchNs / BARS).padStart(12)}`);
console.log(
  '\nStreaming crosses the boundary once per bar, with scalars. Batch crosses it\n' +
    'once per run, but marshals six full arrays to do it -- which of the two wins\n' +
    'is a property of the language, not of the engine behind both of them.\n' +
    'Machine-dependent — compare bindings on one machine, not across machines.',
);
