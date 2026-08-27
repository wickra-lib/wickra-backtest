# Security Policy

## Supported versions

This project is pre-1.0 (alpha). Security fixes are applied to the latest
released version, `0.1.0`, only; please upgrade to the newest release before
reporting an issue.

| Version | Supported |
|---------|-----------|
| 0.1.0 (latest) | ✅ |
| < 0.1.0 | ❌ |

## Reporting a vulnerability

Please report security issues privately to **support@wickra.org**, or via
GitHub's private vulnerability reporting on this repository. Do not open a
public issue for security problems.

Include the affected version, a description, and a reproduction if possible.
We aim to acknowledge reports within a few days.

`wickra-backtest` runs entirely locally: it does not place orders, hold API keys
or connect to exchanges. It is a backtesting library — backtest results are not
indicative of future performance.

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

