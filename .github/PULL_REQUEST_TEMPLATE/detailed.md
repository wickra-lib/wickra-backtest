<!--
Thanks for contributing to wickra-backtest!

This is the long form, for changes that touch the engine, the ABI or more than
one binding. For anything smaller the default template is the right one -- open
the PR without ?template=detailed.md.

Fill in what applies and delete the rest.
-->

## Summary

<!-- 1-3 sentences: what does this change, and why? -->

## Type of change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that changes an existing public API)
- [ ] Performance improvement
- [ ] Refactor (no functional change)
- [ ] Documentation only
- [ ] CI / build / tooling

## Affected surfaces

- [ ] Engine (`crates/wickra-backtest-core`)
- [ ] Data layer (`crates/wickra-backtest-data`)
- [ ] Facade crate (`crates/wickra-backtest`)
- [ ] CLI (`crates/wickra-backtest-cli`)
- [ ] C ABI (`bindings/c`) — the hub every non-native binding calls through
- [ ] Python (`bindings/python`)
- [ ] Node.js (`bindings/node`)
- [ ] WASM (`bindings/wasm`)
- [ ] C# (`bindings/csharp`)
- [ ] Go (`bindings/go`)
- [ ] Java (`bindings/java`)
- [ ] R (`bindings/r`)
- [ ] Golden corpus (`golden/`)
- [ ] Examples / docs

## Linked issues

<!-- "Closes #123", "Refs #456". One per line. -->

Closes #

## How was this tested?

<!--
- Unit tests added or updated (engine tests prefer a hand-computed expectation)
- Golden corpus: which bindings did you re-run?
- Fuzz targets touched? (`fuzz/`)
- Manual repro steps, if applicable
-->

## Engine correctness (if you changed how a bar is processed)

The engine's two claims are that a backtest and a live run are one code path,
and that nothing sees a price before it happened. Both are cheap to break.

- [ ] Streaming and batch produce the identical report on the same bars
- [ ] No look-ahead: nothing consults a value the bar has not produced yet, and
      an order placed on bar `t` fills no earlier than bar `t + 1`
- [ ] Warm-up is respected — no signal before every indicator has its window
- [ ] Edge cases covered: empty series, a single bar, gaps in `time`, a position
      still open at the end

## Output changes (if the report moved)

<!--
An intentional change to the numbers means reblessing the corpus:

    python golden/gen_cases.py                                # only if inputs changed
    WICKRA_BLESS=1 cargo test -p wickra-backtest-core --test golden

Then re-run every binding's golden test against the new expected reports.
-->

- [ ] The change to the numbers is intentional and explained above
- [ ] `golden/expected/` reblessed and the diff reviewed value by value
- [ ] Every binding's golden test re-run and passing

## ABI or surface changes (if you touched `bindings/c` or added an export)

- [ ] `bindings/c/include/wickra_backtest.h` regenerated with cbindgen and committed
- [ ] `bindings/go/include/wickra_backtest.h` updated to match (it is a vendored copy)
- [ ] Every binding grew the new export, or `scripts/check_binding_surface.py`
      records why it has not yet
- [ ] `bindings/c/README.md` documents it — the header is generated, that file is not

## Performance impact (if applicable)

| Benchmark | Before | After | Δ |
| --------- | ------ | ----- | - |
|           |        |       |   |

## Checklist

- [ ] `cargo fmt --all` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` are clean
- [ ] `cargo test --workspace --all-features` passes
- [ ] The changed binding's own suite was run (not only the Rust tests)
- [ ] `scripts/check_binding_surface.py`, `check_version_sync.py`,
      `check_r_abi_skew.py`, `check_readme_links.py` and
      `check_license_copies.py` all pass
- [ ] `CHANGELOG.md` updated under `[Unreleased]`
- [ ] Public API changes are reflected in rustdoc, the READMEs and the examples
- [ ] No local-only notes (`todo*.md`, drafts) staged
- [ ] Licensing unchanged (MIT OR Apache-2.0)

## Notes for reviewers

<!-- What to look at first, known follow-ups, deliberately out-of-scope items. -->
