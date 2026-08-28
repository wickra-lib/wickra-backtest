<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514" alt="Wickra Backtest — backtest and live are byte-identical" width="100%"></a>
</p>

[![Built on Wickra](https://img.shields.io/badge/built%20on-wickra-3b82f6)](https://github.com/wickra-lib/wickra)
[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/ci.svg)](https://github.com/wickra-lib/wickra-backtest/actions/workflows/ci.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-backtest)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/license.svg)](https://github.com/wickra-lib/wickra-backtest#license)

# Wickra Backtest — Node.js

---

Node.js binding for the [wickra-backtest](https://github.com/wickra-lib/wickra-backtest) engine, built with
napi-rs. It runs a strategy spec over OHLCV arrays and returns the report as a
JSON string — byte-identical to the Rust, Python, WASM, C#, Java, Go, C/C++ and
R bindings: one engine kernel behind every language.

## Requirements

- Node.js 18+
- The native addon (built from this crate with napi-rs)

## Build

```bash
cd bindings/node
npm install
npm run build      # compiles the Rust addon and regenerates index.js / index.d.ts
```

## Usage

```js
const { run } = require('.'); // or: import { run } from 'wickra-backtest'

const open  = [100, 102, 104, 98];
const high  = [101, 103, 104, 98];
const low   = [100, 102,  99, 97];
const close = [101, 103,  99, 97];
const volume = [0, 0, 0, 0];
const time   = [0, 1, 2, 3];

const spec = JSON.stringify({
  symbol: 'x', timeframe: '1h', indicators: {},
  entry: { gt: [{ price: 'close' }, 100] },
  exit:  { lt: [{ price: 'close' }, 100] },
  sizing: { type: 'fixed_qty', qty: 1 },
});

const report = JSON.parse(run(open, high, low, close, volume, time, spec, 10_000));
console.log(report.metrics);
```

`run(open, high, low, close, volume, time, specJson, capital?)` returns the
`BacktestReport` as a JSON string (`symbol`, `timeframe`, `metrics`, `trades`,
`equity`, `fees_paid`, `initial_capital`); `capital` defaults to 10,000. An
invalid spec throws.

For strategies that use microstructure feeds, `runJson(requestJson)` takes one
request bundle (candles + spec + optional order-book / trade / derivatives /
cross-section / reference feeds) and returns the same report JSON:

```js
const { runJson } = require('.');
const report = JSON.parse(runJson(JSON.stringify({ capital: 1000, spec, candles, books })));
```

See the [microstructure guide](https://github.com/wickra-lib/wickra-backtest/blob/main/docs/MICROSTRUCTURE.md) for the feed shapes.

The same strategy also runs one bar at a time, which is what makes a backtest and
a live loop the same code path -- swap the array for a socket and nothing else
changes:

```js
const { StreamingBacktest } = require('.');

const bt = new StreamingBacktest(specJson, 10000);
for (const bar of feed) {
  bt.step(bar.open, bar.high, bar.low, bar.close, bar.volume, bar.time);
  console.log(bt.numTrades, JSON.parse(bt.latestEquityJson()));
}
const report = JSON.parse(bt.finishJson());
```

`volume` defaults to 0 and `time` to the number of bars fed so far. Strategies
that read a side feed drive the run with `stepJson` instead, passing
`{ candle, feeds }` per bar. `finishJson()` ends the run; `close()` discards one
without producing a report.

## Test

```bash
npm test
```

## Documentation

- **Repository:** <https://github.com/wickra-lib/wickra-backtest>
- **Strategy spec reference:** [STRATEGY_SPEC.md](https://github.com/wickra-lib/wickra-backtest/blob/main/docs/STRATEGY_SPEC.md)
- **Cookbook:** [COOKBOOK.md](https://github.com/wickra-lib/wickra-backtest/blob/main/docs/COOKBOOK.md)
- **Built on Wickra:** <https://github.com/wickra-lib/wickra> · <https://docs.wickra.org>

The same `StrategySpec` runs identically across Rust, Python, Node.js, WASM, C,
C++, C#, Go, Java and R — one engine kernel, byte-identical reports.

## Security

Found a security issue? **Please don't open a public issue.** Report it privately
via the repository's *Security* tab (*"Report a vulnerability"*) or email
**support@wickra.org**. Full policy:
<https://github.com/wickra-lib/wickra-backtest/blob/main/SECURITY.md>.

## Disclaimer

Not a trading system. Backtest results are deterministic transforms of the input
data — they are not financial advice and are not indicative of future
performance. Any use in a live trading context is at your own risk. Provided
**as is**, without warranty of any kind.

## License

Licensed under either of [MIT](https://github.com/wickra-lib/wickra-backtest/blob/main/LICENSE-MIT) or
[Apache-2.0](https://github.com/wickra-lib/wickra-backtest/blob/main/LICENSE-APACHE) at your option.
