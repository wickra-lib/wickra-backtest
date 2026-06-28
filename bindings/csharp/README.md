<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514" alt="Wickra — streaming-first technical indicators" width="100%"></a>
</p>

[![Built on Wickra](https://img.shields.io/badge/built%20on-wickra-3b82f6)](https://github.com/wickra-lib/wickra)
[![CI](https://github.com/wickra-lib/wickra-backtest/actions/workflows/ci.yml/badge.svg)](https://github.com/wickra-lib/wickra-backtest/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/wickra-lib/wickra-backtest/branch/main/graph/badge.svg)](https://codecov.io/gh/wickra-lib/wickra-backtest)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT_OR_Apache--2.0-blue)](../../README.md#license)

# Wickra Backtest — C#

---

C# binding for the [wickra-backtest](../../README.md) engine. It calls the stable
**C ABI** through P/Invoke, so the results are byte-identical to the Rust,
Python, Node.js, WASM, Java, Go, C/C++ and R bindings: one engine kernel behind
every language.

## Requirements

- .NET 8 SDK
- The native library `wickra_backtest` (built from the C-ABI crate)

## Build the native library

```bash
cargo build -p wickra-backtest-c          # debug   -> target/debug
cargo build -p wickra-backtest-c --release # release -> target/release
```

The test project copies the native library next to the test assembly; for your
own app, ensure `wickra_backtest.dll` / `.so` / `.dylib` is on the load path.

## Usage

```csharp
using Wickra.Backtest;

double[] open  = { 100.0, 102.0, 104.0, 98.0 };
double[] high  = { 101.0, 103.0, 104.0, 98.0 };
double[] low   = { 100.0, 102.0,  99.0, 97.0 };
double[] close = { 101.0, 103.0,  99.0, 97.0 };

string spec =
    "{\"symbol\":\"x\",\"timeframe\":\"1h\",\"indicators\":{}," +
    "\"entry\":{\"gt\":[{\"price\":\"close\"},100]}," +
    "\"exit\":{\"lt\":[{\"price\":\"close\"},100]}," +
    "\"sizing\":{\"type\":\"fixed_qty\",\"qty\":1}}";

string reportJson = Backtester.Run(open, high, low, close, spec: spec, capital: 10_000.0);
Console.WriteLine(reportJson);
```

`Backtester.Run(open, high, low, close, volume?, time?, spec, capital)` returns
the `BacktestReport` JSON string. `volume`/`time` default to zeros / `0..n`. An
invalid spec throws `InvalidOperationException`; no panic crosses the boundary.

## Test

```bash
dotnet test Wickra.Backtest.Tests/Wickra.Backtest.Tests.csproj
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
