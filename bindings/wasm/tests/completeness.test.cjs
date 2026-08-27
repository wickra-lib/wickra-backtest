'use strict';

// Parity guard: the WASM binding exposes exactly the public surface it declares.
//
// The assertions are exact-set rather than "these names exist", because drift
// runs both ways. A dropped export breaks callers; an export that only this
// binding has is a portability lie -- the same strategy would stop being
// runnable everywhere, which is the whole premise.
//
// The streaming surface matches the Node binding's, deliberately: both are
// consumed from JavaScript, so a developer moving between the two packages
// should not have to relearn it. `free` is the one addition -- wasm-bindgen
// generates it to reclaim the instance's memory, which a garbage-collected
// binding has no equivalent for. `close` ends the run; `free` releases the
// object.
//
// Build first with
//   wasm-pack build --target nodejs --out-dir pkg
// then run:  node --test tests/completeness.test.cjs
const test = require('node:test');
const assert = require('node:assert');
const wasm = require('../pkg/wickra_backtest_wasm.js');

const EXPORTS = ['StreamingBacktest', 'run', 'run_json', 'version'];
const STREAM_MEMBERS = [
  'close',
  'equityJson',
  'finishJson',
  'free',
  'latestEquityJson',
  'step',
  'stepJson',
];
// Exposed as getters rather than methods, as in the Node binding.
const STREAM_GETTERS = ['isFinished', 'numTrades'];

test('wasm module exports exactly the declared surface', () => {
  // `__wasm` is the wasm-bindgen instance handle, not part of the API.
  const exported = Object.keys(wasm)
    .filter((n) => !n.startsWith('__'))
    .sort();
  assert.deepStrictEqual(exported, EXPORTS);
  for (const name of ['run', 'run_json', 'version']) {
    assert.strictEqual(typeof wasm[name], 'function', `missing export ${name}`);
  }
  assert.strictEqual(typeof wasm.StreamingBacktest, 'function');
});

test('wasm streaming class exposes exactly the declared members', () => {
  // `__destroy_into_raw` is wasm-bindgen plumbing behind `free`.
  const members = Object.getOwnPropertyNames(wasm.StreamingBacktest.prototype)
    .filter((n) => n !== 'constructor' && !n.startsWith('__'))
    .sort();
  assert.deepStrictEqual(members, [...STREAM_MEMBERS, ...STREAM_GETTERS].sort());
  for (const name of STREAM_MEMBERS) {
    assert.strictEqual(
      typeof wasm.StreamingBacktest.prototype[name],
      'function',
      `missing member ${name}`,
    );
  }
  for (const name of STREAM_GETTERS) {
    const descriptor = Object.getOwnPropertyDescriptor(
      wasm.StreamingBacktest.prototype,
      name,
    );
    assert.ok(
      descriptor && typeof descriptor.get === 'function',
      `missing getter ${name}`,
    );
  }
});
