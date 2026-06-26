'use strict';

// The WASM binding asserts its output against the shared golden reports
// (golden/expected/), pinning cross-language equality. Build first with
//   wasm-pack build --target nodejs --out-dir pkg
// then run:  node --test tests/golden.test.cjs
const test = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const wasm = require('../pkg/wickra_backtest_wasm.js');

test('wasm golden parity', () => {
  const golden = path.join(__dirname, '..', '..', '..', 'golden');
  const casesDir = path.join(golden, 'cases');
  const files = fs.readdirSync(casesDir).filter((f) => f.endsWith('.json'));
  assert.ok(files.length > 0, 'no golden cases found');
  for (const f of files) {
    const c = JSON.parse(fs.readFileSync(path.join(casesDir, f), 'utf8'));
    const got = wasm.run(
      Float64Array.from(c.open),
      Float64Array.from(c.high),
      Float64Array.from(c.low),
      Float64Array.from(c.close),
      Float64Array.from(c.volume),
      Float64Array.from(c.time),
      JSON.stringify(c.spec),
      c.capital,
    );
    const want = fs.readFileSync(path.join(golden, 'expected', `${c.name}.json`), 'utf8').trim();
    assert.strictEqual(got, want, `golden mismatch for ${c.name}`);
  }
});
