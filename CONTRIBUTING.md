# Contributing

Thanks for your interest in `wickra-backtest` — the streaming-native backtester
for the [Wickra](https://github.com/wickra-lib/wickra) indicator library.

## Project layout

| Path | What it is |
| --- | --- |
| `crates/wickra-backtest-core` | The engine: spec, rules, fill model, portfolio, metrics, indicator registry. Everything correctness-critical lives here. |
| `crates/wickra-backtest-data` | Loaders (CSV, JSON, JSON Lines, Parquet), resampling and the Renko / Kagi / Point-and-Figure transforms. |
| `crates/wickra-backtest` | The facade users depend on. It re-exports the core surface as a glob, so it cannot drift out of step with it. |
| `crates/wickra-backtest-cli` | The `wkbt` binary. |
| `crates/wickra-backtest-bench` | Criterion benchmarks. |
| `bindings/c` | The C ABI. Every non-Rust binding goes through it, so a change here is a change to eight languages at once. |
| `bindings/{python,node,wasm}` | Native bindings that link the core directly (PyO3, napi-rs, wasm-bindgen). |
| `bindings/{csharp,go,java,r}` | Bindings over the C ABI. |
| `golden/` | The corpus every binding replays. `cases/` and `requests/` are inputs, `expected/` and `expected_json/` are the pinned outputs. |
| `schema/` | The JSON Schema for `StrategySpec`, generated from the code and drift-tested. |
| `fuzz/` | A detached workspace with five libFuzzer targets. |

## Building and testing

```bash
cargo build --workspace
cargo test --workspace --all-features
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo deny check
```

Run a backtest locally:

```bash
cargo run --bin wkbt -- run --data examples/sample.csv --spec examples/ema-cross.json
```

### Per binding

Each binding has its own suite and CI runs all of them. The commands below are
the ones CI uses; anything that links the C ABI needs it built first.

```bash
cargo build -p wickra-backtest-c --release

# Python (PyO3)
( cd bindings/python && maturin develop && pytest tests -q )

# Node (napi-rs)
( cd bindings/node && npm install && npx napi build --platform --release && node --test )

# WASM (wasm-bindgen)
wasm-pack build bindings/wasm --target nodejs --release --out-dir pkg
node --test bindings/wasm/tests/*.test.cjs

# Go (cgo) -- the C ABI must be on the library path
( cd bindings/go && go vet ./... && go test ./... )

# C# (P/Invoke)
dotnet test bindings/csharp/Wickra.Backtest.Tests/Wickra.Backtest.Tests.csproj -c Release

# Java (FFM)
mvn -B -f bindings/java test

# R (.Call)
Rscript bindings/r/tests/run_tests.R
```

Two things are easy to trip over:

- `bindings/go/include/wickra_backtest.h` is a **copy** of the C ABI header and
  CI diffs the two. Change the ABI and you must copy the header across, or the Go
  job fails with a stale-header error.
- `bindings/r` builds against a downloaded release asset by default. Set
  `WKBT_INC` and `WKBT_LIB` to build against a locally built C ABI instead, and
  put the library directory on `PATH` (Windows) or `LD_LIBRARY_PATH` /
  `DYLD_LIBRARY_PATH` — see `bindings/r/configure`.

A binding that grows a method the C ABI does not export, or loses one it does,
fails `scripts/check_binding_surface.py`, which CI runs as `binding-surface`.

## Lockfile policy

| Component | Lockfile | Tracked? | Why |
| --- | --- | --- | --- |
| Workspace (Rust) | `Cargo.lock` | **yes** | The workspace ships binaries (`wkbt`, the fuzz harness) and CI builds from it, so the graph is pinned for reproducible builds. |
| `bindings/node` | `package-lock.json` | **yes** | Pins the build toolchain for the native binding, and the six platform packages are pinned to the exact version alongside it. CI installs with `npm install`, not `npm ci`: those six have never been published, so they cannot appear in the lock, and `npm ci` refuses a lock that omits a declared dependency. It becomes usable once the first release publishes them. |
| `bindings/python` | — | n/a | The published package declares `dependencies = []`; there is nothing to lock at runtime, and the native code is pinned through the workspace `Cargo.lock`. |
| `.github/requirements` | `ci-dev-py3.txt`, `ci-dev-py39.txt` | **yes** | Hash-pinned dev tooling for the Python job; CI installs them with `--require-hashes`, so the toolchain is a pinned dependency rather than whatever the index served that morning. Split by Python version because 3.9 needs backports later versions do not. |
| `fuzz` | — | n/a | `fuzz/` is a detached crate with no committed lock; the smoke job resolves fresh, which is fine because it proves the targets still build rather than reproducing a byte-identical binary. |

Regenerate every one of them with `./scripts/update-lockfiles.sh`, which needs
[uv](https://docs.astral.sh/uv/) for the Python locks.

When adding a committed Node package, commit its `package-lock.json` too. Do
**not** add a top-level `package-lock.json` — the repository root is not an npm
package.

## Before opening a PR

- `cargo fmt --all` and `cargo clippy … -D warnings` must be clean (CI enforces
  this on three operating systems and the MSRV).
- Add tests. The engine is correctness-critical — prefer a hand-computed
  expectation (see `engine.rs` tests) over a smoke test.
- One logical change per PR; a clear, imperative commit message.
- The default pull-request template is deliberately short. If the change touches
  the engine, the C ABI or more than one binding, use the long form instead --
  add `&template=detailed.md` to the PR-creation URL. It carries the checklists
  that matter there: streaming-versus-batch equality, look-ahead, re-blessing
  the golden corpus, and regenerating the C header and its vendored copy.

## Releasing

Only maintainers tag releases, but the process is written down because it is
irreversible: a tag publishes to crates.io, PyPI, npm, NuGet and Maven Central,
and none of those can be taken back.

**The version lives in twenty-odd declarations** across six package managers --
the workspace manifest and its internal dependency pins, the Python, Node, R,
Java and C# manifests, the six per-platform npm packages, the Node lockfile and
generated loader, the Java example, and `SECURITY.md`. Missing one produces a
release that installs a package pinning a binary that was never published, and
that surfaces on a user's machine rather than in CI. So the first step is the
audit, which CI also runs on every pull request:

```bash
python3 scripts/check_version_sync.py            # every declaration agrees
python3 scripts/check_version_sync.py --previous 0.1.0   # and none is stale
```

It checks declarations, not prose: the `e.g. 0.1.0` in the issue templates is an
illustration and is deliberately not tracked.

### The flow

1. Bump every declaration, run `cargo build` so `Cargo.lock` follows, and
   regenerate the Node artefacts (`npm run build` in `bindings/node`).
2. Move the `[Unreleased]` heading in `CHANGELOG.md` to the new version with the
   date, and add the two comparison links at the bottom of the file.
3. Run the audit above with `--previous <the version you came from>`.
4. Open a pull request. Everything is still reversible up to here.
5. After it merges, tag the merge commit explicitly by SHA -- not `main`, which
   may have moved -- and push the tag:

   ```bash
   git tag -s v0.1.0 <merge-sha> -m "v0.1.0"
   git push origin v0.1.0
   ```

6. Once the publish jobs are green, drop the pre-release wording. Three places
   say "not released yet" and none of them updates itself:

   - the **Status** section of `README.md`, which currently ends "Not yet
     released to any registry";
   - the hand-written **status badge** at the top of `README.md`
     (`status-alpha%20(WIP)`) -- the seventeen registry badges beside it are
     generated from the organisation's badge assets and flip on their own, so
     this is the only badge that needs touching;
   - the repository **description** on GitHub, which ends "(WIP)".

   Being visibly unreleased is currently true and should stay until it is not.

### What the tag triggers

Pushing a `v*` tag runs `release.yml`, which publishes the four crates to
crates.io, wheels and an sdist to PyPI, the Node binding and the WASM package to
npm, the C# package to NuGet, the Java binding to Maven Central, mirrors the Go
module to its own repository, and attaches the built C ABI libraries, `.crate`
files and CycloneDX SBOMs to a GitHub Release with build provenance attestations.
Every publish step is idempotent, so a re-run after a partial failure is safe;
what is not safe is a wrong version, because the registries will not accept a
replacement.

### R is published by registration, not by the tag

`release.yml` has no R job, and that is correct: r-universe *pulls*. It builds
`bindings/r` from this repository's default branch once the package is listed in
the organisation's registry, so the listing is the whole R release path.

The registry lives in the `wickra-lib.r-universe.dev` repository as a single
`packages.json`. Adding this project is one entry alongside the indicator
library's:

```json
{
  "package": "wickrabacktest",
  "url": "https://github.com/wickra-lib/wickra-backtest",
  "subdir": "bindings/r"
}
```

Two things have to be true before that listing is worth making, and both are
checked here rather than discovered days later in the registry's build log:

- `R CMD check` must be clean, which is why `bindings/r/man` is generated and
  committed rather than left to roxygen at build time.
- The wrapper must link against the C ABI of the version `DESCRIPTION` names,
  which `scripts/check_r_abi_skew.py` asserts on every pull request. r-universe
  compiles the wrapper from the default branch against the *published* library,
  a pairing no job in this repository sees otherwise.

### The first release

The version in every manifest is `0.1.0` and has never been tagged, so the first
release is `v0.1.0` itself -- there is no bump to run, only the CHANGELOG move
and the tag. Skipping to `0.1.1` would burn a version number that was never
published.

## Design rules

- **Strategies are data, not code.** The `StrategySpec` is JSON so the same
  strategy runs identically across every Wickra language binding. Keep the DSL
  small and serialisable.
- **No look-ahead bias.** Signals are decided on a bar's close and fill on the
  next bar's open; stop/target/trailing levels fill intrabar. Any change to the
  fill model must preserve this.
- **The engine is feed-agnostic.** It consumes a bar stream; loaders and (later)
  live feeds live outside the core so backtest and live share one engine.

## Indicators

The registry (`registry.rs`) wraps `wickra-core` indicators behind a uniform
`EvalIndicator`. New indicators are added there (and, eventually, generated from
the Wickra manifest). Multi-output indicators expose named fields referenced as
`"name.field"`.

## Developer Certificate of Origin (DCO)

Contributions submitted by pull request are made under the [Developer
Certificate of Origin (DCO) 1.1](DCO). Signing off certifies that you wrote the
patch, or otherwise have the right to submit it under the project's
`MIT OR Apache-2.0` license.

Sign off every commit in your pull request with a `Signed-off-by` trailer
carrying your real name and email — Git adds it with `-s`:

```bash
git commit -s -m "your message"
```

which produces:

```
Signed-off-by: Your Name <you@example.com>
```

The name and email must match the commit author. To sign off a commit you have
already made, amend it with `git commit -s --amend`, or a range with an
interactive rebase.

This is checked when your pull request is reviewed, not by a bot: if a commit is
missing the trailer you will be asked to amend it before the change is merged.
Dependabot adds the trailer to its own commits automatically.

## License

By contributing you agree that your contributions are licensed under the
project's dual `MIT OR Apache-2.0` license.
