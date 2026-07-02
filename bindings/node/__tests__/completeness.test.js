"use strict";

// Parity guard: the Node binding must expose the full public surface of the
// backtester, so an export dropped in a refactor fails loudly here (mirrors the
// completeness check in the main wickra repo).

const { test } = require("node:test");
const assert = require("node:assert");
const wickra = require("../index.js");

const EXPORTS = ["run", "runJson", "version"];

test("Node binding exposes the full public surface", () => {
  for (const name of EXPORTS) {
    assert.strictEqual(
      typeof wickra[name],
      "function",
      `Node binding is missing export ${name}`,
    );
  }
});
