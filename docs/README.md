# docs/

A signpost, not a documentation tree.

The per-language API reference lives at
**[backtest.wickra.org](https://backtest.wickra.org)** — one quickstart per
language surface, generated from the same C ABI every binding sits on:

| | | |
|---|---|---|
| [Rust](https://backtest.wickra.org/api/rust) | [Python](https://backtest.wickra.org/api/python) | [Node.js](https://backtest.wickra.org/api/node) |
| [WebAssembly](https://backtest.wickra.org/api/wasm) | [C](https://backtest.wickra.org/api/c) | [C#](https://backtest.wickra.org/api/csharp) |
| [Go](https://backtest.wickra.org/api/go) | [Java](https://backtest.wickra.org/api/java) | [R](https://backtest.wickra.org/api/r) |

Rust API docs are on [docs.rs](https://docs.rs/wickra-backtest), built with
every feature on.

## What is kept here, and why

Three documents live in this directory rather than on the site, because each
describes something the repository owns and a release changes:

- **[STRATEGY_SPEC.md](STRATEGY_SPEC.md)** — the strategy DSL: operands,
  conditions, sizing, costs, risk. It is the prose counterpart to
  [`schema/strategy_spec.schema.json`](../schema/strategy_spec.schema.json),
  and the two are held together by `scripts/check_example_specs.py`.
- **[COOKBOOK.md](COOKBOOK.md)** — the six strategies under
  [`examples/strategies/`](../examples/strategies/), explained. The files it
  describes are parsed by `tests/example_specs.rs` on every run, so an example
  that stops working fails the build rather than the reader.
- **[MICROSTRUCTURE.md](MICROSTRUCTURE.md)** — replaying order books, trades
  and funding rates, which is what separates this engine from a bar-level
  backtester.

## What does not belong here

Anything that duplicates the site. A second documentation tree in the main
repository is the failure mode this file exists to prevent: it drifts from the
first one, and nobody notices, because both look maintained.

If a page would describe *how to call the library from a language*, it belongs
on the site. If it describes *what the engine accepts or does*, and a release
can change the answer, it belongs beside the code — here.
