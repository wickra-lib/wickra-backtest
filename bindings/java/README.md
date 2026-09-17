<p align="center">
  <a href="https://wickra.org"><img src="https://raw.githubusercontent.com/wickra-lib/.github/main/profile/wickra-banner.webp?v=514-7" alt="Wickra Backtest — backtest and live are byte-identical" width="100%"></a>
</p>

[![CI](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/ci.svg)](https://github.com/wickra-lib/wickra-backtest/actions/workflows/ci.yml)
[![codecov](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/codecov.svg)](https://codecov.io/gh/wickra-lib/wickra-backtest)
[![Maven Central](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/maven.svg)](https://central.sonatype.com/artifact/org.wickra/wickra-backtest)
[![License: MIT OR Apache-2.0](https://raw.githubusercontent.com/wickra-lib/.github/main/profile/badges/wickra-backtest/license.svg)](https://github.com/wickra-lib/wickra-backtest#license)

# Wickra Backtest — Java

---

> **▶ Live demo:** run a strategy in your browser and watch the equity curve build bar by bar — **[backtest-live.wickra.org](https://backtest-live.wickra.org)** · zero backend, the same engine this repository ships, compiled to WebAssembly.

**Backtest and live — for Java. `org.wickra:wickra-backtest` — prebuilt native library inside the jar, no JNI, no system dependencies.**

Java binding for the [wickra-backtest](https://github.com/wickra-lib/wickra-backtest) engine. It calls the
stable **C ABI** through the Java **Foreign Function and Memory API** (FFM,
stable since Java 22 — no preview flags), so the results are byte-identical to
the Rust, Python, Node.js, WASM and C# bindings: one engine kernel behind every
language.

## Requirements

- Java 22+ (tested on 25)
- The native library `wickra_backtest` (built from the C-ABI crate)

## Install

Maven:

```xml
<dependency>
  <groupId>org.wickra</groupId>
  <artifactId>wickra-backtest</artifactId>
  <version>0.1.7</version>
</dependency>
```

Gradle:

```kotlin
implementation("org.wickra:wickra-backtest:0.1.7")
```

The native library ships prebuilt per platform inside the jar and is
extracted automatically on first use. There is nothing to compile.

### Building from this repository (contributors)

**Build the native library.**

```bash
cargo build -p wickra-backtest-c          # debug   -> target/debug
cargo build -p wickra-backtest-c --release # release -> target/release
```

This produces `wickra_backtest.dll` (Windows), `libwickra_backtest.so` (Linux)
or `libwickra_backtest.dylib` (macOS).

**Run the tests.** `mvn test` passes the target directory on `java.library.path` (see `pom.xml`):

```bash
cd bindings/java
mvn test
```

## Quick start

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
(`symbol`, `timeframe`, `metrics`, `trades`, `equity`, `fees_paid`,
`initial_capital`). An invalid spec or mismatched inputs throw
`IllegalStateException` / `IllegalArgumentException`;
no panic ever crosses the FFI boundary.

The same strategy also runs one bar at a time, which is what makes a backtest and
a live loop the same code path -- swap the array for a socket and nothing else
changes:

```java
try (StreamingBacktest bt = new StreamingBacktest(spec, 10_000.0)) {
    for (Bar bar : feed) {
        bt.step(bar.open(), bar.high(), bar.low(), bar.close(), bar.volume(), bar.time());
        System.out.println(bt.numTrades() + " " + bt.latestEquityJson());
    }
    String report = bt.finishJson();
}
```

`StreamingBacktest` owns a native handle, so it is `AutoCloseable` and belongs in
a try-with-resources block. `finishJson()` also releases the handle, and `close()`
afterwards is a no-op, so the two compose. The four-argument `step` uses zero
volume and the bar index as its timestamp. Strategies reading a side feed drive
the run with `stepJson`, passing `{"candle": ..., "feeds": ...}` per bar; using a
finished run throws `IllegalStateException`.

## Benchmark

`benchmarks/` reports this binding's throughput over the shared core. It measures
the call overhead of the Java Foreign Function & Memory API over the C ABI, not a cross-library ratio (the same Rust core runs
under every binding) — see the repository
[BENCHMARKS.md](https://github.com/wickra-lib/wickra-backtest/blob/main/BENCHMARKS.md) for the
numbers, the machine and how each harness is run.

## Documentation

The full guide, the spec reference and the API documentation live in the main
repository and the documentation site:

- **Repository:** <https://github.com/wickra-lib/wickra-backtest>
- **Docs** (guides, spec reference, cookbook): <https://backtest.wickra.org>
- **Runnable example:** [`examples/java/`](https://github.com/wickra-lib/wickra-backtest/tree/main/examples/java)

- **Repository:** <https://github.com/wickra-lib/wickra-backtest>
- **Strategy spec reference:** [STRATEGY_SPEC.md](https://github.com/wickra-lib/wickra-backtest/blob/main/docs/STRATEGY_SPEC.md)
- **Cookbook:** [COOKBOOK.md](https://github.com/wickra-lib/wickra-backtest/blob/main/docs/COOKBOOK.md)
- **Built on Wickra:** <https://github.com/wickra-lib/wickra> · <https://docs.wickra.org>

The same `StrategySpec` runs identically across Rust, Python, Node.js, WASM, C,
C++, C#, Go, Java and R — one engine kernel, byte-identical reports.

Wickra Backtest ships native bindings for Python, Node.js, WASM and Rust, plus a C ABI hub that any
C-capable language (C, C++, C#, Go, Java, R) links against — all forwarding to the
same data-driven, `unsafe`-forbidden Rust core.

## Security

Found a security issue? **Please don't open a public issue.** Report it privately
via the repository's *Security* tab (*"Report a vulnerability"*) or email
**support@wickra.org** with a subject line starting `[wickra security]`. Full
policy: <https://github.com/wickra-lib/wickra-backtest/blob/main/SECURITY.md>.

## Disclaimer

Not a trading system. Backtest results are deterministic transforms of the input
data — they are not financial advice and are not indicative of future
performance. Any use in a live trading context is at your own risk. The software
is provided **as is**, without warranty of any kind; see the license files for
the full terms.

## License

Licensed under either of [Apache-2.0](https://github.com/wickra-lib/wickra-backtest/blob/main/LICENSE-APACHE)
or [MIT](https://github.com/wickra-lib/wickra-backtest/blob/main/LICENSE-MIT) at your option.
