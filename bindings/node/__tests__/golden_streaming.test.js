'use strict';

const { test } = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const { StreamingBacktest } = require('..');

// Driving each shared case one bar at a time must reproduce the same canonical
// report (golden/expected/) the batch entry point produces. golden.test.js pins
// the batch side; this pins that streaming did not drift away from it.
test('streaming golden parity', () => {
  const dir = path.join(__dirname, '..', '..', '..', 'golden');
  const casesDir = path.join(dir, 'cases');
  const files = fs.readdirSync(casesDir).filter((f) => f.endsWith('.json'));
  assert.ok(files.length > 0, 'no golden cases found');
  for (const f of files) {
    const c = JSON.parse(fs.readFileSync(path.join(casesDir, f), 'utf8'));
    const bt = new StreamingBacktest(JSON.stringify(c.spec), c.capital);
    for (let i = 0; i < c.close.length; i++) {
      bt.step(c.open[i], c.high[i], c.low[i], c.close[i], c.volume[i], c.time[i]);
    }
    const want = fs.readFileSync(path.join(dir, 'expected', `${c.name}.json`), 'utf8').trim();
    assert.strictEqual(bt.finishJson(), want, `streaming mismatch for ${c.name}`);
  }
});
