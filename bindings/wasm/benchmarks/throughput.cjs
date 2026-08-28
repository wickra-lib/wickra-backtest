// Throughput benchmark for the wickra-backtest WebAssembly build.
//
// Measures what crossing the boundary costs. Every reach in this repository
// runs the same Rust engine, so a difference between two bindings is not a
// difference in the backtester -- it is the price of that language's FFI, paid
// once per bar on the streaming path and once per run on the batch path.
//
// WASM is the interesting one to compare against the native Node binding: same
// JavaScript caller, same engine, a different boundary. Whatever gap shows up
// between the two is the cost of running in the sandbox rather than in a
// dynamic library.
//
// The strategy is examples/ema-cross.json: two EMAs, a crossover, fractional
// sizing, taker costs, slippage and a trailing stop.
//
// Run after building the package for Node:
//
//   wasm-pack build bindings/wasm --target nodejs --release --out-dir pkg
//   node bindings/wasm/benchmarks/throughput.cjs               # 200k bars
//   node bindings/wasm/benchmarks/throughput.cjs --bars 1000000

const wasm = require('../pkg/wickra_backtest_wasm.js');

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

// Deterministic synthetic OHLCV, from the same formula every binding's harness
// uses. Typed arrays because the batch entry point takes Float64Array.
const open = new Float64Array(BARS);
const high = new Float64Array(BARS);
const low = new Float64Array(BARS);
const close = new Float64Array(BARS);
const volume = new Float64Array(BARS);
const time = new Float64Array(BARS);
for (let i = 0; i < BARS; i++) {
  const mid = 100 + Math.sin(i * 0.001) * 20 + i * 1e-4;
  close[i] = mid + Math.sin(i * 0.05) * 2;
  open[i] = i ? close[i - 1] : close[i];
  high[i] = Math.max(open[i], close[i]) + 1.5;
  low[i] = Math.min(open[i], close[i]) - 1.5;
  volume[i] = 1000 + (i % 97) * 13;
  time[i] = i;
}

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
  const bt = new wasm.StreamingBacktest(SPEC, CAPITAL);
  for (let i = 0; i < BARS; i++) {
    bt.step(open[i], high[i], low[i], close[i], volume[i], time[i]);
  }
  bt.finishJson();
  // wasm-bindgen owns the instance's linear memory; finish leaves the wrapper
  // inert but the allocation is reclaimed here, not by the collector.
  bt.free();
});

const batchNs = timeNs(() => {
  wasm.run(open, high, low, close, volume, time, SPEC, CAPITAL);
});

const fmt = (n) => Math.round(n).toLocaleString('en-US');
console.log(`wickra-backtest WASM throughput — ${fmt(BARS)} bars (median of 3 runs)\n`);
console.log(`${'path'.padEnd(14)}${'bars/sec'.padStart(16)}${'ns/bar'.padStart(12)}`);
console.log('-'.repeat(42));
console.log(`${'streaming'.padEnd(14)}${fmt(barsPerSecond(streamingNs)).padStart(16)}${fmt(streamingNs / BARS).padStart(12)}`);
console.log(`${'batch'.padEnd(14)}${fmt(barsPerSecond(batchNs)).padStart(16)}${fmt(batchNs / BARS).padStart(12)}`);
console.log(
  '\nStreaming crosses the boundary once per bar, with scalars. Batch crosses it\n' +
    'once per run, copying six Float64Arrays into linear memory to do it.\n' +
    'Run bindings/node/benchmarks/throughput.js on the same machine for the\n' +
    'native comparison — the difference is the sandbox, not the engine.',
);
