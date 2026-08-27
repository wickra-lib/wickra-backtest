---
name: Bug report
about: Report incorrect behaviour or a crash.
title: "[Bug] <short description>"
labels: ["bug", "triage"]
assignees: []
---

## What happened

<!-- The observed behaviour. A wrong number is a bug; say which number. -->

## What you expected

<!-- And why -- a hand-computed value, another tool's output, the documented contract. -->

## Reproduction

<!--
The smallest strategy spec and slice of candle data that shows it. A spec that
reproduces in `wkbt` is worth more than a prose description.
-->

```json
{ "spec_version": 1, "symbol": "BTCUSDT", "timeframe": "1h", "indicators": [] }
```

```bash
wkbt run --data slice.csv --spec spec.json
```

## Environment

- wickra-backtest version: `e.g. 0.1.0`
- Binding: `Rust / Python / Node.js / WASM / C / C++ / C# / Go / Java / R`
- OS / arch: `e.g. Linux 6.8 x86_64`

## Additional context

<!-- Report output, stack trace, anything you already ruled out. -->
