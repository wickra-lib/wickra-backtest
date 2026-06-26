# Wickra Backtest — Node.js

Node.js binding for the [wickra-backtest](../../README.md) engine, built with
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
`BacktestReport` as a JSON string (`metrics`, `trades`, `equity`, `fees_paid`,
`initial_capital`); `capital` defaults to 10,000. An invalid spec throws.

## Test

```bash
npm test
```

## Documentation

- **Repository:** <https://github.com/wickra-lib/wickra-backtest>
- **Strategy spec reference:** [STRATEGY_SPEC.md](../../docs/STRATEGY_SPEC.md)
- **Cookbook:** [COOKBOOK.md](../../docs/COOKBOOK.md)
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

Licensed under either of [MIT](../../LICENSE-MIT) or
[Apache-2.0](../../LICENSE-APACHE) at your option.
