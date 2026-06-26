'use strict';

// Feed golden parity for the WASM binding: each request bundle
// (golden/requests/) drives a microstructure feed path through run_json, and the
// output is asserted byte-for-byte against the shared expected reports
// (golden/expected_json/). Build first with:
//   wasm-pack build --target nodejs --out-dir pkg
// then run:  node --test tests/golden_json.test.cjs
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const wasm = require('../pkg/wickra_backtest_wasm.js');

test('wasm feed golden parity (run_json)', () => {
  const dir = path.join(__dirname, '..', '..', '..', 'golden');
  const reqDir = path.join(dir, 'requests');
  const files = fs.readdirSync(reqDir).filter((f) => f.endsWith('.json'));
  assert.ok(files.length > 0, 'no golden requests found');
  for (const f of files) {
    const name = f.replace(/\.json$/, '');
    const request = fs.readFileSync(path.join(reqDir, f), 'utf8');
    const got = wasm.run_json(request);
    const want = fs.readFileSync(path.join(dir, 'expected_json', `${name}.json`), 'utf8').trim();
    assert.strictEqual(got, want, `feed golden mismatch for ${name}`);
  }
});
