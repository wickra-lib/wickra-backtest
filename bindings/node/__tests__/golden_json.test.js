'use strict';

const { test } = require('node:test');
const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const { runJson } = require('..');

// Feed golden parity: each request bundle (golden/requests/) exercises a
// microstructure feed path through the unified run_json entry point, and the
// Node binding asserts its output byte-for-byte against the shared expected
// reports (golden/expected_json/), pinning cross-language feed equality.
test('feed golden parity (run_json)', () => {
  const dir = path.join(__dirname, '..', '..', '..', 'golden');
  const reqDir = path.join(dir, 'requests');
  const files = fs.readdirSync(reqDir).filter((f) => f.endsWith('.json'));
  assert.ok(files.length > 0, 'no golden requests found');
  for (const f of files) {
    const name = f.replace(/\.json$/, '');
    const request = fs.readFileSync(path.join(reqDir, f), 'utf8');
    const got = runJson(request);
    const want = fs.readFileSync(path.join(dir, 'expected_json', `${name}.json`), 'utf8').trim();
    assert.strictEqual(got, want, `feed golden mismatch for ${name}`);
  }
});
