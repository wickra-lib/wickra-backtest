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

## Contribution flow

Every change goes through a pull request, including a maintainer's own. That is
what puts CI -- tests across ten language reaches, linting, static analysis --
in front of the change rather than behind it, and what leaves a history someone
else can read. The requirements are in
[CONTRIBUTING.md](CONTRIBUTING.md), including the Developer Certificate of
Origin sign-off that every contributed commit must carry.

## Becoming a maintainer

The project has one maintainer today. Maintainership may be extended to
contributors who have shown sustained, high-quality involvement, at the current
maintainer's discretion. If it grows to several, this document will be updated
to describe how decisions are shared.

## Releases

Releases follow semantic versioning. Pre-1.0, the spec and report schemas may
change between minor versions. A release is tagged `vX.Y.Z` by a maintainer and
published to the language registries by CI.

## Continuity and succession

The project should survive the loss of any single individual, so that issues can
be triaged, changes accepted and releases published within a week of a confirmed
loss of the maintainer. That matters more here than the size of the project
suggests: a tag publishes to six registries in one irreversible run, and a
half-finished release is worse than none.

- **Credentials.** Everything needed to operate the project -- the `wickra-lib`
  GitHub organisation, the publishing credentials for crates.io, PyPI, npm,
  Maven Central and the Go module mirror, and the `wickra.org` registrar -- is
  held in a password manager. A trusted contact holds emergency access to it.
  NuGet needs no stored credential: it is published through OIDC trusted
  publishing, which is tied to this repository and its workflow rather than to a
  person.
- **Continuity actions.** With that access, the trusted contact or a delegate
  they appoint can triage issues, accept pull requests and cut releases through
  the existing workflows. Nothing about a release requires a maintainer's own
  machine; the release runs entirely in CI from a signed tag.
- **Account recovery.** The maintainer's GitHub account has recovery configured,
  and ownership of the `wickra-lib` organisation can be transferred.
- **Legal rights.** Rights to the project name and DNS are covered by the
  maintainer's estate arrangements.

## Code of conduct

Everyone taking part is expected to follow the
[Code of Conduct](CODE_OF_CONDUCT.md).

## Changes to governance

This document is changed by a pull request approved by the maintainers.
