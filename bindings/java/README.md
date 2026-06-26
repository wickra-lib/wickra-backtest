# Wickra Backtest — Java

Java binding for the [wickra-backtest](../../README.md) engine. It calls the
stable **C ABI** through the Java **Foreign Function and Memory API** (FFM,
stable since Java 22 — no preview flags), so the results are byte-identical to
the Rust, Python, Node.js, WASM and C# bindings: one engine kernel behind every
language.

## Requirements

- Java 22+ (tested on 25)
- The native library `wickra_backtest` (built from the C-ABI crate)

## Build the native library

```bash
cargo build -p wickra-backtest-c          # debug   -> target/debug
cargo build -p wickra-backtest-c --release # release -> target/release
```

This produces `wickra_backtest.dll` (Windows), `libwickra_backtest.so` (Linux)
or `libwickra_backtest.dylib` (macOS).

## Run the tests

`mvn test` passes the target directory on `java.library.path` (see `pom.xml`):

```bash
cd bindings/java
mvn test
```

## Usage

Put the native library on `java.library.path`
(`-Djava.library.path=/path/to/target/debug`) and call:

```java
import org.wickra.backtest.Backtester;

double[] open  = { 100.0, 102.0, 104.0, 98.0 };
double[] high  = { 101.0, 103.0, 104.0, 98.0 };
double[] low   = { 100.0, 102.0,  99.0, 97.0 };
double[] close = { 101.0, 103.0,  99.0, 97.0 };

String spec =
    "{\"symbol\":\"x\",\"timeframe\":\"1h\",\"indicators\":{},"
  + "\"entry\":{\"gt\":[{\"price\":\"close\"},100]},"
  + "\"exit\":{\"lt\":[{\"price\":\"close\"},100]},"
  + "\"sizing\":{\"type\":\"fixed_qty\",\"qty\":1}}";

String reportJson = Backtester.run(open, high, low, close, spec); // capital defaults to 10,000
System.out.println(reportJson);
```

`run` overloads:

- `run(open, high, low, close, spec)` — zero volume, `0..n` timestamps, capital 10,000
- `run(open, high, low, close, spec, capital)`
- `run(open, high, low, close, volume, time, spec, capital)` — full control

The returned JSON is the same `BacktestReport` as every other binding
(`metrics`, `trades`, `equity`, `fees_paid`, `initial_capital`). An invalid spec
or mismatched inputs throw `IllegalStateException` / `IllegalArgumentException`;
no panic ever crosses the FFI boundary.

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
