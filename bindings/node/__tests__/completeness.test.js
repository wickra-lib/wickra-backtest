"use strict";

// Parity guard: the Node binding must expose the full public surface of the
// backtester, so an export dropped in a refactor fails loudly here (mirrors the
// completeness check in the main wickra repo).

const { test } = require("node:test");
const assert = require("node:assert");
const wickra = require("../index.js");

const EXPORTS = ["run", "runJson", "version"];

// The streaming class is the binding's other half: a dropped method would
// leave `run` working and the "backtest and live are one code path" claim
// quietly false, which no value-comparing test would catch.
const STREAM_METHODS = [
  "step",
  "stepJson",
  "equityJson",
  "latestEquityJson",
  "finishJson",
  "close",
];
const STREAM_GETTERS = ["numTrades", "isFinished"];

test("Node binding exposes the full public surface", () => {
  for (const name of EXPORTS) {
    assert.strictEqual(
      typeof wickra[name],
      "function",
      `Node binding is missing export ${name}`,
    );
  }
});

test("Node binding exposes the full streaming surface", () => {
  assert.strictEqual(
    typeof wickra.StreamingBacktest,
    "function",
    "Node binding is missing StreamingBacktest",
  );
  const proto = wickra.StreamingBacktest.prototype;
  for (const name of STREAM_METHODS) {
    assert.strictEqual(
      typeof proto[name],
      "function",
      `StreamingBacktest is missing ${name}`,
    );
  }
  for (const name of STREAM_GETTERS) {
    const descriptor = Object.getOwnPropertyDescriptor(proto, name);
    assert.ok(
      descriptor && typeof descriptor.get === "function",
      `StreamingBacktest is missing the ${name} getter`,
    );
  }
});
