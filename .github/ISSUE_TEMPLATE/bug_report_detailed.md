---
name: Bug report (detailed)
about: A bug that needs more than a paragraph -- wrong fills, drifting equity, a binding disagreeing with Rust.
title: "[Bug] <short description>"
labels: ["bug", "triage"]
assignees: []
---

## Summary

<!-- One or two sentences. What is wrong, and how wrong. -->

## Affected area

- [ ] Engine (fills, sizing, costs, intrabar exits)
- [ ] Strategy spec / rule DSL
- [ ] Indicator registry
- [ ] Data loaders or bar transforms
- [ ] A language binding disagreeing with the Rust result
- [ ] CLI (`wkbt`)

## Reproduction

<!-- Minimal spec, minimal data. Attach both if they do not fit inline. -->

```json
{ }
```

```bash
wkbt run --data slice.csv --spec spec.json
```

## Expected vs actual

| | Expected | Actual |
| --- | --- | --- |
| e.g. `trades` | `2` | `3` |
| e.g. `return_pct` | `1.20` | `-0.35` |

<!--
If the expectation is hand-computed, show the arithmetic. That is what makes a
report actionable rather than a difference of opinion about a number.
-->

## Cross-binding check (if relevant)

<!--
Does the Rust result differ from a binding's? All ten go through the same engine
and are pinned by the golden corpus, so a disagreement is a marshalling bug and
worth saying so explicitly.
-->

| Binding | Result |
| --- | --- |
| Rust | |
| The one that differs | |

## Environment

| Field | Value |
| --- | --- |
| wickra-backtest version | `e.g. 0.1.0` |
| Binding | `Rust / Python / Node.js / WASM / C / C++ / C# / Go / Java / R` |
| OS / arch | `e.g. Windows 11 x86_64` |
| Toolchain | `rustc 1.x.y` |
| Features enabled | `e.g. parquet, binance` |

## What you already ruled out

<!-- Saves a round trip. -->
