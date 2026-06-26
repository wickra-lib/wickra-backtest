# Governance

`wickra-backtest` is part of the Wickra project and follows the same lightweight
governance model.

## Roles

- **Maintainers** review and merge changes, cut releases and set direction. The
  current maintainers are listed in [MAINTAINERS.md](MAINTAINERS.md).
- **Contributors** propose changes via pull requests. Anyone may contribute; see
  [CONTRIBUTING.md](CONTRIBUTING.md).

## Decision making

Day-to-day changes are merged by a maintainer once CI is green and the change
has been reviewed. Larger or breaking changes (spec format, report schema,
public API) are discussed in an issue first and decided by maintainer consensus;
the lead maintainer breaks ties.

## Releases

Releases follow semantic versioning. Pre-1.0, the spec and report schemas may
change between minor versions. A release is tagged `vX.Y.Z` by a maintainer and
published to the language registries by CI.

## Changes to governance

This document is changed by a pull request approved by the maintainers.
