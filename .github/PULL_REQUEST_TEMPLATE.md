<!--
Keep it short. One logical change per PR.

For a change that touches the engine, the C ABI or several bindings at once,
there is a long form with the correctness and re-blessing checklists: add
`&template=detailed.md` to the PR-creation URL. GitHub offers no picker for it,
so this line is the only thing that makes it findable.
-->

## What

<!-- What does this change and why? -->

## Checklist

- [ ] `cargo fmt --all` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` are clean
- [ ] `cargo test --workspace --all-features` passes
- [ ] Tests added/updated (prefer hand-computed expectations for engine changes)
- [ ] No look-ahead bias introduced into the fill model
- [ ] `CHANGELOG.md` updated under `[Unreleased]`
