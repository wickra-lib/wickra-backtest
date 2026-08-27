---
name: Performance regression
about: Report a measurable slowdown, memory blowup or throughput drop.
title: "[Perf] <area> regressed in <version>"
labels: ["performance", "regression", "triage"]
assignees: []
---

## Summary

<!-- Which path got slower, by how much, and since when. -->

## Affected path

- Area: `e.g. StreamingBacktest::step, run_json, the CSV loader`
- Binding: `Rust / Python / Node.js / WASM / C / C++ / C# / Go / Java / R`
- Per-bar hot loop, or a one-shot run?

## Versions compared

| Version | Throughput / latency / memory | Notes |
| --- | --- | --- |
| `0.1.0` | `e.g. 1.2 ms / 50k bars` | baseline |
| `0.1.1` | `e.g. 4.8 ms / 50k bars` | regressed |

## Benchmark / reproducer

<!-- The command and its output. For a one-off measurement, include the timing snippet. -->

```bash
cargo bench -p wickra-backtest-bench
```

```
```

## Hardware / environment

| Field | Value |
| --- | --- |
| CPU | `e.g. Ryzen 9 9950X` |
| OS / arch | `e.g. Linux 6.8 x86_64` |
| Toolchain | `rustc 1.x.y` |
| Features | `e.g. parquet` |

## Suspected cause

<!-- Optional. Link the commit or pull request if you bisected it. -->
