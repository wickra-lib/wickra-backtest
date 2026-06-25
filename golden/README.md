# Golden parity corpus

This directory is the **cross-language contract** for the backtester: a set of
strategy cases and the single canonical report each one must produce. Every
language binding runs the same cases and asserts its output against the same
expected reports, so "backtest = live = identical across languages" is pinned
and regression-proof.

## Layout

```
golden/
  gen_cases.py        generates the input cases (deterministic data + specs)
  cases/<name>.json   one case: { name, capital, spec, open/high/low/close/volume/time }
  expected/<name>.json  the canonical report JSON, produced by the Rust engine
```

The cases exercise the breadth of the engine: a no-indicator price threshold,
a two-EMA crossover, an RSI mean-reversion, and a MACD long/short strategy
using multi-output `"name.field"` references.

## How it works

1. `gen_cases.py` writes the input cases (run it only when changing inputs).
2. The Rust integration test (`crates/wickra-backtest-core/tests/golden.rs`) is
   the **anchor**: it runs each case through the engine and, in bless mode,
   writes `expected/<name>.json`. The expected report is exactly
   `serde_json::to_string(&report)` — the same bytes every JSON-returning
   binding emits.
3. Each binding has a golden test that loads the cases, runs them and compares
   to `expected/`. JSON-returning bindings (Node, Go, C, C++, C#, Java, R, WASM)
   match **byte-for-byte**; the Python binding returns a dict and matches the
   parsed expected JSON **value-for-value**.

## Regenerating the expected reports

After an intentional engine change that alters output:

```bash
python golden/gen_cases.py                                   # only if inputs changed
WICKRA_BLESS=1 cargo test -p wickra-backtest-core --test golden
```

Then re-run every binding's golden test; they must all pass against the new
expected reports.

## Wiring a binding in

Read `cases/*.json`, call the binding's `run(open, high, low, close, volume,
time, spec, capital)`, and compare to `expected/<name>.json`:

| Binding | Test | Match |
|---------|------|-------|
| Rust    | `crates/wickra-backtest-core/tests/golden.rs` | anchor (writes expected) |
| Go      | `bindings/go/golden_test.go`        | byte-for-byte |
| Node.js | `bindings/node/__tests__/golden.test.js` | byte-for-byte |
| Python  | `bindings/python/tests/test_golden.py`   | value-for-value (dict) |
| C# / Java / R / C / C++ / WASM | same pattern | byte-for-byte |
