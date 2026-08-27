'use strict';

// Run the shared EMA-cross strategy from Node, both ways.
//
//   node examples/node/backtest.js
//
// Reads the same examples/sample.csv and examples/ema-cross.json every other
// language example uses, runs the whole series at once, then feeds the same bars
// one at a time and checks that the two agree. That equality is the point of the
// library: a live loop is the streaming path with a socket in place of the file,
// so a backtest is not a separate model of the strategy.
//
// Requires the built binding: `npm run build` in bindings/node.

const fs = require('node:fs');
const path = require('node:path');
const { run, StreamingBacktest } = require('../../bindings/node');

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

  const batch = run(
    bars.map((b) => b.open),
    bars.map((b) => b.high),
    bars.map((b) => b.low),
    bars.map((b) => b.close),
    bars.map((b) => b.volume),
    bars.map((b) => b.time),
    spec,
    CAPITAL,
  );

  // The same run, driven bar by bar. Replace the loop with reads from a socket
  // and this is a live strategy; nothing else about it changes.
  const live = new StreamingBacktest(spec, CAPITAL);
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
