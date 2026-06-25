<!-- Keep it short. One logical change per PR. -->

## What

<!-- What does this change and why? -->

## Checklist

- [ ] `cargo fmt --all` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` are clean
- [ ] `cargo test --workspace --all-features` passes
- [ ] Tests added/updated (prefer hand-computed expectations for engine changes)
- [ ] No look-ahead bias introduced into the fill model
- [ ] `CHANGELOG.md` updated under `[Unreleased]`
