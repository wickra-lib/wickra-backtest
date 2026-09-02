# Security Policy

## Supported versions

This project is pre-1.0 (alpha). Security fixes are applied to the latest
released version, `0.1.3`, only; please upgrade to the newest release before
reporting an issue.

| Version | Supported |
|---------|-----------|
| 0.1.3 (latest) | ✅ |
| < 0.1.3 | ❌ |

## Reporting a vulnerability

Please report security issues privately to **support@wickra.org**, or via
GitHub's private vulnerability reporting on this repository. Do not open a
public issue for security problems.

Include the affected version, a description, and a reproduction if possible.
We aim to acknowledge reports within a few days.

`wickra-backtest` runs entirely locally: it does not place orders, hold API keys
or connect to exchanges. It is a backtesting library — backtest results are not
indicative of future performance.

## What to expect

- An acknowledgement within **5 working days**.
- An assessment and, if confirmed, a planned fix with a target release.
- Coordinated disclosure: we will agree a disclosure date with you and credit
  you in the release notes unless you would rather stay anonymous.

## Scope

In scope: the published crates (`wickra-backtest-core`, `wickra-backtest-data`,
`wickra-backtest`, `wickra-backtest-cli`), the packages built from this
repository for PyPI, npm, NuGet, Maven Central, the Go module mirror and
r-universe, the C ABI in [`bindings/c`](bindings/c), and the build and release
workflows under `.github/workflows/`.

Out of scope: vulnerabilities in third-party dependencies. Report those
upstream; we track them with Dependabot and `cargo-deny`.

## Security assurance case

A short, evidence-backed argument for why this can be used safely.

**Security requirements.** This is a backtesting engine. It ingests numeric
market data and a strategy described as a JSON document, and produces a report.
It stores no credentials, authenticates nobody, and implements no cryptography.
The requirements that follow are: (1) memory safety and freedom from undefined
behaviour, (2) robust handling of untrusted input -- and the strategy spec is
untrusted input, not just the prices -- without panics or unbounded resource
use, (3) integrity of the published artefacts, and (4) a healthy dependency
supply chain.

**How they are met.**

- *Memory safety* — the engine and every binding are Rust. The workspace sets
  `unsafe_code = "forbid"`, and the four published crates restate it in their
  own `lib.rs`, so the compiler guarantees memory and thread safety for
  everything that computes. The exception is the C ABI, whose shim is
  necessarily `unsafe` because it dereferences caller-supplied pointers; it
  contains no engine logic, checks every pointer for null, and catches panics at
  each boundary so none crosses into a foreign runtime.
- *Input robustness* — the spec parser is the largest attack surface here,
  because a strategy arrives as a document rather than as code. Five
  coverage-guided fuzz targets run in CI against exactly the paths that read
  untrusted bytes: the spec parser, the JSON request entry point, the engine
  loop, the fill model and the CSV loader.
- *Static and dynamic analysis* — every push and pull request runs Clippy
  (`clippy::pedantic`, warnings as errors), CodeQL, `zizmor` against the
  workflows themselves, the fuzz smoke suite and the full test suite across ten
  language reaches. Line coverage on the engine is tracked by Codecov.
- *Artefact integrity* — releases are built in CI and never from a maintainer's
  machine, every commit and tag in the history is signed, and the crates and
  Python distributions carry build provenance attestations. The C ABI, on which
  six of the ten reaches depend, is built once per platform in CI and consumed
  from those artefacts rather than rebuilt per publisher.
  rather than rebuilt per publisher.
- *Supply chain* — dependencies are pinned, watched by Dependabot across every
  manifest that has an external dependency, and audited by `cargo-deny` for
  advisories and licences on every change. The CI tooling is installed from
  hash-locked requirement files.

**Residual risk.** The optional `binance` feature opens a TLS connection to an
exchange through the platform TLS library, so transport security there depends
on that library rather than on this project. A strategy spec is data, and the
engine executes no code from it -- but it can describe an arbitrarily large
indicator set, so an untrusted spec should be resource-limited by the caller.
This is not a trading system and is provided "as is"; see the disclaimers in
`README.md` and the licences.

## Secrets management

No secrets or credentials are stored in version control. What automation needs
is held as **GitHub Actions encrypted secrets**, referenced through the
`secrets.*` context, and never written to the repository, the logs or a build
artefact. Publishing to NuGet uses **OIDC trusted publishing**, so that registry
needs no long-lived key at all; the others use scoped registry tokens. GitHub
**secret scanning with push protection** is enabled, so a credential committed
by accident is blocked rather than merely reported. Secrets are granted the
narrowest scope that works and are rotated when a holder changes or on suspected
exposure.

## Verifying releases

Released artifacts can be verified for integrity and authenticity:

- **Build provenance.** The `.crate` files, Python wheels and the sdist carry
  GitHub build provenance attestations, produced by the release workflow and
  attached to the GitHub Release as a bundle. Verify a downloaded asset with the
  GitHub CLI: `gh attestation verify <file> --repo wickra-lib/wickra-backtest`.
  The Node and WASM packages carry npm's own provenance instead, published with
  `npm publish --provenance` and shown on the package page.
- **Signed tags.** Each release corresponds to a signed git tag (`vX.Y.Z`); the
  tag signature identifies the maintainer who authorised the release.
- **Registry integrity.** Packages are distributed over HTTPS from crates.io,
  PyPI, npm, NuGet and Maven Central, which serve checksums that package
  managers verify on install.

The release is published only by the maintainer through the tag-triggered
release workflow, so a verified tag signature establishes the expected publisher
identity.

## Support timeline and end of support

Only the **latest released version** receives security fixes. When a newer
release is published the previous one **immediately reaches end of support** and
will not receive further fixes; upgrade to the latest release. The
supported-versions table above is authoritative. A defined support window
covering older releases may be introduced later; until then, only the latest
release is supported.

## Remediation policy (dependencies and code scanning)

- **Severity threshold.** Vulnerabilities of **medium severity or higher**, in
  this project's own code or in a dependency, are remediated promptly and before
  the next release. Lower-severity findings are addressed on a best-effort basis.
- **Automated enforcement (SCA).** Every change is evaluated by `cargo-deny`
  (RUSTSEC advisories and the licence policy) and by Dependabot. A
  known-vulnerable dependency fails CI and **blocks the change** until it is
  resolved, or waived with a written justification in `deny.toml`.
- **Automated enforcement (SAST).** Every change is evaluated by CodeQL and by
  Clippy with warnings as errors; the workflows themselves are evaluated by
  `zizmor`. Findings **block the change** in CI until they are fixed.
- **Pre-release gate.** A release is not cut while an unresolved
  medium-or-higher SCA or SAST finding is outstanding. Because a tag publishes to
  six registries in one irreversible run, this gate is checked before tagging
  rather than after.

## Vulnerability exploitability (VEX)

Advisories reported by `cargo-deny`, OSV-Scanner or Dependabot for third-party
dependencies that do **not** affect this project — the vulnerable code path is
unreachable, or the affected feature is off by default and not enabled here — are
triaged and recorded with the not-affected justification rather than forcing an
unnecessary dependency bump.

Two files hold that record, and they are kept in lock-step:

- `deny.toml`, in the `[advisories] ignore` list, for the Rust dependency graph.
- `osv-scanner.toml`, as `[[IgnoredVulns]]` entries, which is what the OpenSSF
  Scorecard Vulnerabilities check reads.

Every entry carries a reason. An advisory is never suppressed to make a counter
reach zero; if the assessment is that the project *is* affected, the dependency
is bumped or the feature dropped instead.

