<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514" alt="Wickra Backtest — backtest and live are byte-identical" width="100%"></a>
</p>

[![Built on Wickra](https://img.shields.io/badge/built%20on-wickra-3b82f6)](https://github.com/wickra-lib/wickra)
[![Status](https://img.shields.io/badge/status-alpha%20(WIP)-orange)](https://github.com/wickra-lib/wickra-backtest)
[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/ci.svg)](https://github.com/wickra-lib/wickra-backtest/actions/workflows/ci.yml)
[![CodeQL](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/codeql.svg)](https://github.com/wickra-lib/wickra-backtest/actions/workflows/codeql.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-backtest)
[![GitHub release](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/release.svg)](https://github.com/wickra-lib/wickra-backtest/releases/latest)
[![crates.io](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/crates.svg)](https://crates.io/crates/wickra-backtest)
[![PyPI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/pypi.svg)](https://pypi.org/project/wickra-backtest/)
[![npm](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/npm.svg)](https://www.npmjs.com/package/wickra-backtest)
[![NuGet](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/nuget.svg)](https://www.nuget.org/packages/Wickra.Backtest)
[![Maven Central](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/maven.svg)](https://central.sonatype.com/artifact/org.wickra/wickra-backtest)
[![Go module](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/go.svg)](https://pkg.go.dev/github.com/wickra-lib/wickra-backtest-go)
[![R-universe](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/r-universe.svg)](https://wickra-lib.r-universe.dev)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/license.svg)](#license)
[![OpenSSF Scorecard](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/scorecard.svg)](https://scorecard.dev/viewer/?uri=github.com/wickra-lib/wickra-backtest)
[![OpenSSF Best Practices](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/best-practices.svg)](https://www.bestpractices.dev/)
[![Build provenance](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/provenance.svg)](https://github.com/wickra-lib/wickra-backtest/attestations)
[![Docs](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/docs.svg)](https://backtest.wickra.org)
[![Verified across 10 languages](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/verified.svg)](golden/)
[![Live demo](https://img.shields.io/badge/live%20demo-backtest--live.wickra.org-3b82f6)](https://backtest-live.wickra.org)

---

**Backtest and live — byte-identical, in 10 languages.** A streaming-native,
event-driven backtester built on the [Wickra](https://github.com/wickra-lib/wickra)
indicator core.

> **▶ Live demo:** run a strategy in your browser and watch the equity curve build bar by bar — **[backtest-live.wickra.org](https://backtest-live.wickra.org)** · zero backend, the same engine this repository ships, compiled to WebAssembly.

> **Part of the [Wickra ecosystem](https://github.com/wickra-lib):** the same data-driven core and ten-language binding surface also power [wickra-exchange](https://github.com/wickra-lib/wickra-exchange), [wickra-terminal](https://github.com/wickra-lib/wickra-terminal), [wickra-screener](https://github.com/wickra-lib/wickra-screener), [wickra-xray](https://github.com/wickra-lib/wickra-xray), [wickra-radar](https://github.com/wickra-lib/wickra-radar), [wickra-copilot](https://github.com/wickra-lib/wickra-copilot) and [wickra-shazam](https://github.com/wickra-lib/wickra-shazam).

The engine consumes the **exact same `wickra-core` O(1) indicator kernels** that
power live Wickra, and a strategy is **data (a JSON spec), not code** — so a
backtest and a live run over the same spec produce identical signals, across
every Wickra language binding. The same engine, fed live instead of historical
bars, becomes the live bot: **backtest ≡ live, by construction.**

The strategy spec can reference **495 `wickra-core` indicators** by name — every
backtestable scalar, candle, multi-output, pairwise, derivatives, order-book,
trade, trade-quote and cross-section indicator, with multi-output fields
addressed as `"name.field"` (`macd.signal`, `bb.upper`, `adx.plus_di`, …). The
registry is generated directly from the wickra-core sources, so it stays in
lock-step with the kernel.

```bash
pip install wickra-backtest
```

```python
import wickra_backtest as wbt

spec = {                                  # a strategy is data, not code
    "symbol": "BTCUSDT", "timeframe": "1h",
    "indicators": {"fast": {"type": "Ema", "params": [12]},
                   "slow": {"type": "Ema", "params": [26]}},
    "entry": {"cross_above": ["fast", "slow"]},
    "exit":  {"cross_below": ["fast", "slow"]},
    "sizing": {"type": "fixed_fraction", "fraction": 0.95},
}

# Backtest: the whole series at once.
report = wbt.run(opens, highs, lows, closes, spec=spec)
print(report["metrics"]["return_pct"], report["metrics"]["sharpe"])

# Live: the same spec, the same engine, one bar at a time. Point `step` at a
# socket instead of an array and nothing else changes.
with wbt.StreamingBacktest(spec=spec) as live:
    for bar in feed:
        live.step(bar.open, bar.high, bar.low, bar.close)
        print(live.num_trades, live.latest_equity())
    report = live.finish()
```

The two reports are byte-identical. That is the whole claim, and a shared
[golden corpus](golden/) holds every one of the ten bindings to it.

What it does differently:

- **O(1) per tick** — years of tick data in seconds, not hours (no recompute-on-every-tick).
- **Backtest = live, value-identical across 10 languages** — no reimplementation drift, pinned by a shared golden corpus for the OHLCV path *and* every microstructure feed.
- **Microstructure backtesting** — replay the order book, trades, perpetual funding and open interest as strategy inputs, not just OHLCV. Most Python backtesters have no place to put them.
- **Realistic execution** — long/short, market/limit/stop orders, leverage and position caps, five sizing models, intrabar stop-loss / take-profit / trailing stops, maker/taker fees, three slippage models, perpetual funding, liquidation and execution latency.
- **Polyglot** — the same `StrategySpec` runs from Rust, Python, Node.js, WASM, C, C++, C#, Go, Java and R.

| Backtester | Languages | Engine | Strategy is | Book / funding inputs | Latest release |
|------------|-----------|--------|-------------|----------------------|----------------|
| **★&nbsp;wickra-backtest** | **Rust · Python · Node.js · WASM · C · C++ · C# · Go · Java · R** | **event-driven, O(1)/bar** | **data (a JSON spec)** | **yes** | unreleased |
| nautilus_trader | Rust · Python | event-driven | code | yes | 2026-08 |
| vectorbt | Python | vectorised | code | — | 2026-07 |
| backtesting.py | Python | vectorised | code | — | 2026-07 |
| zipline-reloaded | Python | event-driven | code | — | 2025-07 |
| backtrader | Python | event-driven | code | — | 2023-04 |

Release dates are the latest published version on PyPI, checked when this table
was written; "—" means the feed is not a first-class strategy input, not that the
library is bad at what it does. nautilus_trader is the closest comparison and is
ahead of this project in places — it is a full trading platform with live venue
adapters, and it has shipped for years. The distinction here is narrower and
worth stating plainly: a strategy is **data rather than code**, so the same spec
runs unchanged from ten languages and a shared golden corpus pins every one of
them to the same report, byte for byte. No other engine in this table offers that
because none of them needs to.

## Status

**Alpha / work in progress.** The engine, the data-driven `StrategySpec`, the
full execution and cost model, the microstructure feeds and all ten language
bindings are implemented and tested; a shared [golden corpus](golden/) pins the
cross-language equality byte-for-byte. Not yet released to any registry.

## Documentation

- **[Strategy spec reference](docs/STRATEGY_SPEC.md)** — the full DSL: operands,
  conditions, sizing, costs, slippage, risk, execution and the report shape.
- **[Cookbook](docs/COOKBOOK.md)** — six ready-to-run strategies (RSI mean
  reversion, MACD trend, Bollinger breakout, Donchian breakout, funding carry,
  order-book imbalance), each validated against the engine.
- **[Microstructure guide](docs/MICROSTRUCTURE.md)** — backtesting on the order
  book, trades, perpetual funding and market breadth (the differentiator).
- **[Architecture](ARCHITECTURE.md)** — crates, data flow and design decisions.
- **[Benchmarks](BENCHMARKS.md)** — throughput methodology and caveats.
- **[Examples](examples/)** — runnable specs and a sample dataset.
- The JSON Schema for the spec is at
  [`schema/strategy_spec.schema.json`](schema/strategy_spec.schema.json) and is
  printed by `wkbt schema`.

## Quickstart

A strategy is **data** — a JSON spec. Run one over a candle file with the `wkbt` CLI:

```bash
cargo run --bin wkbt -- run --data examples/sample.csv --spec examples/ema-cross.json
```

```text
bars       80
trades     4
return     -2.68%
pnl        -268.21
sharpe     -0.186
max dd     2.68%
win rate   0.0%
fees       37.50
```

A spec declares named indicators and entry/exit rules over them:

```json
{
  "symbol": "BTCUSDT", "timeframe": "1h",
  "indicators": { "ema_fast": { "type": "Ema", "params": [5] },
                  "ema_slow": { "type": "Ema", "params": [15] } },
  "entry": { "cross_above": ["ema_fast", "ema_slow"] },
  "exit":  { "cross_below": ["ema_fast", "ema_slow"] },
  "sizing": { "type": "fixed_fraction", "fraction": 0.95 },
  "risk": { "trailing_stop_pct": 5.0 }
}
```

See the [cookbook](docs/COOKBOOK.md) and [`examples/`](examples/) for complete
strategies, and the [spec reference](docs/STRATEGY_SPEC.md) for the full grammar.

From Rust, the same thing is `wickra_backtest::run(&spec, &candles)`. For live
use, `StreamingBacktest::new(&spec, capital)` then `step(candle)` per bar feeds
the **same engine** one bar at a time — backtest and live are one code path. A
single `run_json` request bundles candles, the spec and any feeds, and is the
uniform entry point every binding wraps.

## Run the same spec in any language

Every binding takes the same OHLCV arrays (or a `run_json` request) and JSON spec
and returns the same report — byte-identical (a dict in Python). Each has a
quickstart:

| Binding | Install | Example |
|---------|---------|---------|
| Rust | `cargo add wickra-backtest` | [`examples/rust`](examples/rust/src/main.rs) |
| Python (PyO3) | `pip install wickra-backtest` | [`examples/python/backtest.py`](examples/python/backtest.py) |
| Node.js (napi-rs) | `npm install wickra-backtest` | [`examples/node/backtest.js`](examples/node/backtest.js) |
| Browser / WASM | `npm install wickra-backtest-wasm` | [`examples/wasm/backtest.cjs`](examples/wasm/backtest.cjs) |
| C / C++ (C ABI) | header + library, see [`bindings/c`](bindings/c/README.md) | [`examples/c/streaming.c`](examples/c/streaming.c) |
| C# (C ABI) | `dotnet add package Wickra.Backtest`, see [`bindings/csharp`](bindings/csharp/README.md) | [`examples/csharp`](examples/csharp/Program.cs) |
| Go (cgo, C ABI) | `go get github.com/wickra-lib/wickra-backtest-go`, see [`bindings/go`](bindings/go/README.md) | [`examples/go`](examples/go/backtest.go) |
| Java (FFM, C ABI) | Maven Central `org.wickra:wickra-backtest`, see [`bindings/java`](bindings/java/README.md) | [`examples/java`](examples/java/src/main/java/org/wickra/backtest/examples/Backtest.java) |
| R (`.Call`, C ABI) | `R CMD INSTALL bindings/r`, see [`bindings/r`](bindings/r/README.md) | [`examples/r/backtest.R`](examples/r/backtest.R) |

Every example does the same thing in its own language: read the shared sample
data, run the series both ways, and fail if the two reports differ.
[`examples/README.md`](examples/README.md) is the cross-language index.

The C, C++, C#, Go, Java and R bindings all call through the same C ABI hub; the
[golden corpus](golden/) asserts every language produces the same report, for
both the plain OHLCV path and the order-book / trade / derivatives /
cross-section feed paths.

## Performance

O(1) per bar — about **1.7M bars/second** on one core (a year of 1-minute bars in
~0.3 s). See [BENCHMARKS.md](BENCHMARKS.md) for the methodology and caveats.

## Project layout

```
wickra-backtest/
├── crates/
│   ├── wickra-backtest-core/   engine: spec DSL, registry, rules, execution, portfolio, metrics, report
│   ├── wickra-backtest-data/   loaders (CSV / JSON / JSONL / Parquet) + resampling + Renko/Kagi/PnF
│   ├── wickra-backtest/        facade crate (re-exports the engine + runners)
│   ├── wickra-backtest-cli/    the `wkbt` command-line backtester
│   └── wickra-backtest-bench/  criterion throughput benchmarks
├── bindings/
│   ├── python/   PyO3 + maturin          ├── csharp/  P/Invoke over the C ABI
│   ├── node/     napi-rs                 ├── go/      cgo over the C ABI
│   ├── wasm/     wasm-bindgen            ├── java/    FFM over the C ABI
│   ├── c/        C ABI (cdylib/staticlib + generated header)
│   └── r/        .Call over the C ABI
├── golden/       shared cross-language parity corpus (cases + feed requests)
├── schema/       generated JSON Schema for the strategy spec
├── examples/     runnable strategies + a sample dataset
├── docs/         strategy spec reference + cookbook
└── fuzz/         cargo-fuzz targets (nightly)
```

## Building from source

```bash
# Rust core + tests + lints
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo bench -p wickra-backtest-bench

# Python binding (requires a Rust toolchain + maturin)
cd bindings/python && maturin develop --release && pytest

# Node binding (requires @napi-rs/cli)
cd bindings/node && npm install && npm run build && npm test

# WASM binding (requires wasm-pack)
cd bindings/wasm && wasm-pack build --target nodejs --out-dir pkg && node --test tests/

# C ABI (cdylib + staticlib + generated header)
cargo build -p wickra-backtest-c --release

# C# binding (requires the .NET 8 SDK; links the C ABI above)
dotnet test bindings/csharp/Wickra.Backtest.Tests/Wickra.Backtest.Tests.csproj

# Go binding (requires a C compiler for cgo; links the C ABI above)
cd bindings/go && go test ./...

# Java binding (requires JDK 22+ and Maven; links the C ABI above)
mvn -f bindings/java test

# R binding (requires a C toolchain / Rtools; links the C ABI above)
WKBT_INC="$PWD/bindings/c/include" WKBT_LIB="$PWD/target/debug" R CMD INSTALL bindings/r
```

The Go, Java and R bindings load the C ABI shared library at run time; put
`target/debug` (or `target/release`) on the library path. Fuzzing requires a
nightly toolchain — see [`fuzz/`](fuzz/); the same never-panic invariants are
covered on stable by the property tests.

## Testing

Every layer is covered; the commands are in
[Building from source](#building-from-source).

- `wickra-backtest-core` — 113 unit tests: hand-computed round trips (entry at
  the next open, exit on the signal after it), every sizing model, the cost and
  slippage models, intrabar stops and liquidation, funding, the rule evaluator,
  bounded history, and the feed requirements a spec declares. Plus five
  integration suites: property tests, the shipped example specs, and the three
  golden runners (batch, streaming, and the microstructure requests).
- `wickra-backtest-data` — 17 unit tests over CSV, JSONL, JSON-array and Binance
  kline decoding, plus the resamplers.
- `bindings/c` — 12 Rust tests driving the ABI itself, including the streaming
  handle's lifecycle and every error path, so a null or finished handle is
  proven to be reported rather than dereferenced.
- `bindings/python` — 21 pytest cases: smoke, streaming, golden parity,
  completeness of the module and class surface, and the feed path.
- `bindings/node` — 19 `node --test` cases, same shape.
- `bindings/wasm` — 8 `node --test` cases against the built package.
- `bindings/csharp` — 15 xUnit cases. `bindings/java` — 15 JUnit cases.
  `bindings/go` — 15 `go test` cases. `bindings/r` — 3 script suites.
- `fuzz/` — five targets covering the whole untrusted-input surface: the spec
  parser, the JSON request, the engine loop, the fill model and the data loader.

On top of those, **all ten languages** replay a shared, language-neutral golden
corpus — four OHLCV cases and five microstructure requests in `golden/` — and
assert equality with the Rust reference report. Since the streaming work, each
also replays the corpus **one bar at a time** and asserts the same report, so
the claim that a backtest and a live loop agree is pinned per language rather
than argued.

> **What "parity" means here, precisely.** The reports are compared
> **byte for byte**, not to a tolerance. That is possible because every binding
> calls the same Rust engine — the arithmetic is not reimplemented anywhere — and
> because the indicators the corpus names use only IEEE-754 arithmetic, which
> every conforming platform rounds identically. It is not free, though: a spec
> can name any indicator in the core, and some of those call a transcendental
> from the platform's math library (`ln`, `atan`, `exp` and friends). No
> mainstream libm rounds those correctly, and implementations differ in the last
> bit — the sibling indicator library measured a one-ulp difference on 24 of 67
> bars for a single indicator. A golden case built on one of those would have to
> compare to a relative tolerance instead. None currently does, and that is a
> property of the corpus worth keeping deliberately rather than by accident.

## Requirements

The minimum supported version per language. The same engine kernel runs behind
every binding; the C-ABI bindings that compile on install — Go (cgo) and R
(`.Call`) — also need a C compiler, and Java runs with
`--enable-native-access=ALL-UNNAMED`.

| Language | Package                                   | Minimum supported          |
|----------|-------------------------------------------|----------------------------|
| Rust     | crates.io · `wickra-backtest`             | 1.86 (MSRV)                |
| Python   | PyPI · `wickra-backtest` (abi3 wheel)     | 3.9 (tested through 3.13)  |
| Node.js  | npm · `wickra-backtest` (N-API 8)         | 22 (tested on 22 · 24 LTS) |
| WASM     | npm · `wickra-backtest-wasm`              | any modern JS engine       |
| C        | `wickra_backtest.h` + library (releases)  | C99 compiler               |
| C++      | over the C ABI                            | C++14 compiler             |
| C#       | NuGet · `Wickra.Backtest`                 | .NET 8 (`net8.0`)          |
| Go       | module · `wickra-lib/wickra-backtest-go`  | Go 1.23 (cgo)              |
| Java     | Maven Central · `org.wickra:wickra-backtest` | Java 22 (FFM / Panama)  |
| R        | r-universe · `wickrabacktest`             | R ≥ 2.10 (Rtools on Win.)  |

## Ecosystem

Part of the [Wickra](https://github.com/wickra-lib/wickra) family — each one a
data-driven core with a CLI and the same ten-language binding surface:

- [**wickra**](https://github.com/wickra-lib/wickra) — the core library: 514 O(1) streaming indicators across ten languages
- [**wickra-exchange**](https://github.com/wickra-lib/wickra-exchange) — unified market-data + execution across ten crypto exchanges
- [**wickra-backtest**](https://github.com/wickra-lib/wickra-backtest) — event-driven backtester over the Wickra core
- [**wickra-terminal**](https://github.com/wickra-lib/wickra-terminal) — the trading terminal: a TUI and a browser renderer over the stack
- [**wickra-screener**](https://github.com/wickra-lib/wickra-screener) — parallel multi-symbol screening over 514 streaming indicators
- [**wickra-xray**](https://github.com/wickra-lib/wickra-xray) — market-microstructure explorer: footprint, order-book heatmap, liquidation map, funding/OI divergence
- [**wickra-radar**](https://github.com/wickra-lib/wickra-radar) — perp-universe alert radar: OI delta, funding flip, book imbalance, liquidation clusters, OI/price divergence
- [**wickra-copilot**](https://github.com/wickra-lib/wickra-copilot) — local market copilot grounded in real order-book, liquidation and funding microstructure
- [**wickra-shazam**](https://github.com/wickra-lib/wickra-shazam) — match an asset's current microstructure fingerprint against its entire history

This project's own site is [backtest.wickra.org](https://backtest.wickra.org) and
its in-browser demo [backtest-live.wickra.org](https://backtest-live.wickra.org).
The indicator core it is built on documents itself at
[docs.wickra.org](https://docs.wickra.org).

## Contributing

Contributions are welcome — issues, bug reports, ideas and pull requests all land
at <https://github.com/wickra-lib/wickra-backtest>. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the orientation: the engine lives in
`crates/wickra-backtest-core`, every binding under `bindings/<lang>` keeps the
golden-corpus parity invariant, and `cargo fmt --all` +
`cargo clippy --workspace --all-targets --all-features -- -D warnings` are CI
gates. For larger changes, open an issue first.

## Security

Found a security issue? **Please don't open a public issue.** Report it privately
via the repository's *Security* tab (*"Report a vulnerability"*) or email
**support@wickra.org**. Full policy: [SECURITY.md](SECURITY.md).

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option. Use it, fork it, modify it, redistribute it — commercially or
not — file issues, send pull requests; all welcome.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

## Disclaimer

Not a trading system. Backtest results are deterministic transforms of the input
data — they are not financial advice and are not indicative of future
performance. Any use in a live trading context is at your own risk. The software
is provided **as is**, without warranty of any kind; see the license files for
the full terms.

---

<p align="center">
  Built on <a href="https://github.com/wickra-lib/wickra">Wickra</a>. If it saved you time, ⭐ the repo.
</p>
