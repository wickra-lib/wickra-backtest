# Wickra Backtest — C#

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
