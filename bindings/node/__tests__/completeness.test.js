"use strict";

// Parity guard: the Node binding exposes exactly the public surface of the
// backtester (mirrors the completeness check in the main wickra repo).
//
// The assertions are exact-set rather than "these names exist", because drift
// runs both ways. A dropped export breaks callers; an export that only this
// binding has is a portability lie -- the same strategy would stop being
// runnable everywhere, which is the whole premise. Either direction fails here.

const { test } = require("node:test");
const assert = require("node:assert");
const wickra = require("../index.js");

const EXPORTS = ["StreamingBacktest", "run", "runJson", "version"];

// The streaming class is the binding's other half: a dropped method would leave
// `run` working and the "backtest and live are one code path" claim quietly
// false, which no value-comparing test would catch.
const STREAM_METHODS = [
  "close",
  "equityJson",
  "finishJson",
  "latestEquityJson",
  "step",
  "stepJson",
];
const STREAM_GETTERS = ["isFinished", "numTrades"];

test("Node binding exports exactly the declared surface", () => {
  assert.deepStrictEqual(Object.keys(wickra).sort(), EXPORTS);
  for (const name of ["run", "runJson", "version"]) {
    assert.strictEqual(typeof wickra[name], "function", `missing export ${name}`);
  }
  assert.strictEqual(typeof wickra.StreamingBacktest, "function");
});

test("Node streaming class exposes exactly the declared members", () => {
  const proto = wickra.StreamingBacktest.prototype;
  const members = Object.getOwnPropertyNames(proto)
    .filter((n) => n !== "constructor")
    .sort();
  assert.deepStrictEqual(members, [...STREAM_METHODS, ...STREAM_GETTERS].sort());

  for (const name of STREAM_METHODS) {
    assert.strictEqual(typeof proto[name], "function", `missing method ${name}`);
  }
  for (const name of STREAM_GETTERS) {
    const descriptor = Object.getOwnPropertyDescriptor(proto, name);
    assert.ok(
      descriptor && typeof descriptor.get === "function",
      `missing getter ${name}`,
    );
  }
});
