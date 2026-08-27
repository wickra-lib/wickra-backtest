'use strict';

// Driving each shared case one bar at a time must reproduce the same canonical
// report (golden/expected/) the batch entry point produces. golden.test.cjs pins
// the batch side; this pins that streaming did not drift away from it.
// Build first with
//   wasm-pack build --target nodejs --out-dir pkg
// then run:  node --test tests/golden_streaming.test.cjs
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const wasm = require('../pkg/wickra_backtest_wasm.js');

test('wasm streaming golden parity', () => {
  const golden = path.join(__dirname, '..', '..', '..', 'golden');
  const casesDir = path.join(golden, 'cases');
  const files = fs.readdirSync(casesDir).filter((f) => f.endsWith('.json'));
  assert.ok(files.length > 0, 'no golden cases found');
  for (const f of files) {
    const c = JSON.parse(fs.readFileSync(path.join(casesDir, f), 'utf8'));
    const bt = new wasm.StreamingBacktest(JSON.stringify(c.spec), c.capital);
    for (let i = 0; i < c.close.length; i++) {
      bt.step(c.open[i], c.high[i], c.low[i], c.close[i], c.volume[i], c.time[i]);
    }
    const got = bt.finishJson();
    const want = fs.readFileSync(path.join(golden, 'expected', `${c.name}.json`), 'utf8').trim();
    assert.strictEqual(got, want, `streaming mismatch for ${c.name}`);
  }
});
