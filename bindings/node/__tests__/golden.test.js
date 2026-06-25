'use strict';

const { test } = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const { run } = require('..');

// Every binding asserts its output against the shared golden reports
// (golden/expected/), pinning cross-language equality. The Node binding returns
// the engine JSON verbatim, so the match is byte-for-byte.
test('golden parity', () => {
  const dir = path.join(__dirname, '..', '..', '..', 'golden');
  const casesDir = path.join(dir, 'cases');
  const files = fs.readdirSync(casesDir).filter((f) => f.endsWith('.json'));
  assert.ok(files.length > 0, 'no golden cases found');
  for (const f of files) {
    const c = JSON.parse(fs.readFileSync(path.join(casesDir, f), 'utf8'));
    const got = run(c.open, c.high, c.low, c.close, c.volume, c.time, JSON.stringify(c.spec), c.capital);
    const want = fs.readFileSync(path.join(dir, 'expected', `${c.name}.json`), 'utf8').trim();
    assert.strictEqual(got, want, `golden mismatch for ${c.name}`);
  }
});
