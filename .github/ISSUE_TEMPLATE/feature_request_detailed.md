---
name: Feature request (detailed)
about: A capability that changes the spec, the engine, or every binding at once.
title: "[Feature] <short description>"
labels: ["enhancement"]
assignees: []
---

## Problem

<!-- The situation this leaves you in today. Concrete beats general. -->

## Proposal

<!-- What you want to exist. -->

## Spec impact

<!--
Strategies are data, not code: anything new has to be expressible in JSON and
serialisable, and it lands in the committed JSON Schema. Sketch the shape.
-->

```json
{ }
```

- [ ] Adds a field to `StrategySpec`
- [ ] Adds an `OrderType`, `Sizing`, `Slippage` or `Condition` variant
- [ ] Needs a new feed family
- [ ] No spec change (engine-internal)

## Binding impact

<!--
Ten languages sit on one C ABI. A change that adds an export is a change to all
of them, and `scripts/check_binding_surface.py` will hold them to it.
-->

- [ ] No new C ABI export
- [ ] Adds a C ABI export, so every binding grows a method

## Backwards compatibility

<!-- Does an existing spec keep producing the same report? If not, say what moves. -->

## Alternatives

<!-- Including "do it outside the engine" and why that is not enough. -->
