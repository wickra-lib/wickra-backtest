# Architecture

`wickra-backtest` is a streaming-native, event-driven backtester built on the
`wickra-core` indicator kernels. A strategy is **data** (a JSON spec), so the
same strategy runs identically in ten languages and a backtest matches a live
run by construction.

## Crates

| Crate | Role |
|-------|------|
| `wickra-backtest-core` | The engine: spec DSL, indicator registry, rule evaluation, portfolio accounting, execution model, metrics, report and the streaming/batch runners. |
| `wickra-backtest-data` | Data loaders (CSV / JSON / JSON Lines / Parquet) and transforms (resampling, Renko / Kagi / Point-and-Figure bars). |
| `wickra-backtest` | A thin facade re-exporting the public API. |
| `wickra-backtest-cli` | The `wkbt` command-line backtester. |
| `wickra-backtest-bench` | Criterion throughput benchmarks. |
| `bindings/*` | Python, Node.js, WASM, C ABI, C#, Go, Java and R language bindings. |

## Data flow

```
candles ──▶ StreamingBacktest::step(candle, feeds)
                 │
                 ├─ 1. fill any pending order (look-ahead-free: on this bar's open)
                 ├─ 2. update each registered indicator from a BarInput
                 │      (candle + optional reference / derivatives / order-book /
                 │       trade / cross-section feeds)
                 ├─ 3. evaluate intrabar stop-loss / take-profit / trailing /
                 │      liquidation along the conservative O→H→L→C path
                 ├─ 3b. charge perpetual funding to an open position
                 ├─ 4. mark-to-market and push an equity point
                 └─ 5. evaluate entry / exit / short rules → queue an order
                        (filled at step 1 of a later bar, after latency)
            finish() ──▶ close any open position ──▶ BacktestReport
```

The historical `run` / `run_with_capital` is exactly this loop fed from a slice;
pointing `step` at a live feed turns the same engine into the live bot. That
single code path is why a backtest equals a live run.

## Key design decisions

- **Look-ahead bias is structural.** A signal computed on a bar's close fills on
  the *next* bar's open by default; the optimistic same-bar `close` fill is
  explicit opt-in. The engine never reads a future bar.
- **The registry is generated.** `tools/gen_registry.py` parses the
  `wickra-core` indicator sources and emits `registry.rs`, so the 495
  backtestable indicators stay in lock-step with the library. Each is wrapped
  behind a uniform, object-safe `EvalIndicator` the engine drives from a
  `BarInput`.
- **Feeds are an extensible bundle.** `Feeds { reference, deriv, orderbook,
  trades, cross_section }` threads microstructure context to the indicators that
  need it; absent feeds are simply `None`. `run_json` marshals one JSON request
  (candles + spec + feeds) so every binding exposes the full surface without
  marshalling variable-length feed arrays across the FFI.
- **One kernel, ten languages.** Every binding calls the same engine and returns
  the same JSON report. The golden corpus (`golden/`) pins this byte-for-byte
  for the OHLCV path and all four microstructure feed families.

## Trust boundaries

The engine is pure computation: no network, no order placement, no API keys.
Inputs (specs, candles, feeds) are untrusted JSON; parsing and evaluation never
panic (see `tests/properties.rs`), always returning a `Result`. See
[THREAT_MODEL.md](THREAT_MODEL.md).
