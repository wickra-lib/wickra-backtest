'use strict';

// Run the shared EMA-cross strategy through the WebAssembly build, both ways.
//
//   wasm-pack build bindings/wasm --target nodejs --out-dir pkg
//   node examples/wasm/backtest.cjs
//
// Reads the same examples/sample.csv and examples/ema-cross.json every other
// language example uses, runs the whole series at once, then feeds the same bars
// one at a time and checks that the two agree. In a browser the loop below is
// driven by a websocket instead of a file, which is the whole difference between
// this and a live strategy.
//
// The streaming surface is the same as the Node binding's, deliberately: both are
// consumed from JavaScript, so moving between the two packages should not mean
// relearning method names.

const fs = require('node:fs');
const path = require('node:path');
const wasm = require('../../bindings/wasm/pkg/wickra_backtest_wasm.js');

const ROOT = path.join(__dirname, '..');
const CAPITAL = 10000;

// The CSV columns are time,open,high,low,close,volume.
function loadBars(file) {
  const [header, ...rows] = fs
    .readFileSync(file, 'utf8')
    .trim()
    .split(/\r?\n/);
  const columns = header.split(',');
  return rows.map((line) => {
    const values = line.split(',').map(Number);
    return Object.fromEntries(columns.map((name, i) => [name, values[i]]));
  });
}

function main() {
  const spec = fs.readFileSync(path.join(ROOT, 'ema-cross.json'), 'utf8');
  const bars = loadBars(path.join(ROOT, 'sample.csv'));

  const batch = wasm.run(
    new Float64Array(bars.map((b) => b.open)),
    new Float64Array(bars.map((b) => b.high)),
    new Float64Array(bars.map((b) => b.low)),
    new Float64Array(bars.map((b) => b.close)),
    new Float64Array(bars.map((b) => b.volume)),
    // Timestamps cross the boundary as doubles: WASM has no native i64 array.
    new Float64Array(bars.map((b) => b.time)),
    spec,
    CAPITAL,
  );

  const live = new wasm.StreamingBacktest(spec, CAPITAL);
  for (const b of bars) {
    live.step(b.open, b.high, b.low, b.close, b.volume, b.time);
  }
  const streamed = live.finishJson();

  const report = JSON.parse(streamed);
  const m = report.metrics;
  console.log(`bars            ${bars.length}`);
  console.log(`trades          ${m.num_trades}`);
  console.log(`pnl             ${m.pnl.toFixed(2)}`);
  console.log(`return %        ${m.return_pct.toFixed(2)}`);
  console.log(`max drawdown    ${m.max_drawdown.toFixed(4)}`);
  console.log(`final equity    ${report.equity[report.equity.length - 1].equity.toFixed(2)}`);

  if (streamed !== batch) {
    console.error('streaming and batch disagree -- that should be impossible');
    process.exit(1);
  }
  console.log('\nstreaming reproduces the batch report exactly');
}

main();
