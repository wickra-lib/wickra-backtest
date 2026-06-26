# Threat model

`wickra-backtest` is a local computation library. It loads market data and a
strategy spec, runs a simulation and returns a report. It does **not** place
orders, hold API keys, connect to exchanges or open network sockets.

## Assets

- The user's machine and process running the backtest.
- The integrity of the backtest result (no silent miscomputation).

## Trust boundaries and inputs

All inputs are **untrusted**:

| Input | Source | Handling |
|-------|--------|----------|
| Strategy spec (JSON) | user / third party | Parsed and validated; never panics, returns a typed error on malformed input. |
| Candle / feed data (CSV, JSON, JSON Lines, Parquet) | user / data vendor | Parsed defensively; malformed rows produce errors, not panics. |
| FFI arguments (bindings) | host language | Null pointers and invalid UTF-8 are checked; no panic crosses the C ABI (`catch_unwind`). |

## Threats considered

- **Malicious or malformed input causing a crash.** Mitigated: the parser and
  engine are property-tested to never panic on arbitrary input
  (`tests/properties.rs`); the C ABI catches any unwind and returns an error
  code rather than aborting the host process.
- **Silently wrong results.** Mitigated: a golden corpus pins the output
  byte-for-byte across all ten language bindings, for both the OHLCV path and
  every microstructure feed family. Indicator values come from `wickra-core`, so
  they equal a live run.
- **Supply-chain risk.** Dependencies are reviewed with `cargo-deny`
  (licenses + advisories) and `osv-scanner`; the dependency tree is intentionally
  small and the heavy `arrow`/`parquet` stack is opt-in behind a feature.
- **Resource exhaustion.** A backtest is O(1) per bar and bounded by the input
  length; there is no unbounded recursion or allocation driven by spec content.

## Out of scope

- **Trading / financial risk.** Backtest results are not indicative of future
  performance. The library models execution as faithfully as it can (fees,
  slippage, funding, liquidation, latency, partial fills) but cannot guarantee
  live fills.
- **Confidentiality.** The library handles no secrets; it does not authenticate,
  encrypt or transmit data.
- **Multi-tenant isolation.** It is a single-process library, not a service.

## Reporting

Security issues: see [SECURITY.md](SECURITY.md) — report privately to
**support@wickra.org** or via GitHub private vulnerability reporting.
