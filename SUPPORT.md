# Support

Thanks for using `wickra-backtest`. Where to go depends on what you need.

## Documentation first

Most questions are already answered:

- **Project site:** <https://backtest.wickra.org>
- **[README](README.md)** — installation for all ten languages, and a first
  strategy in each.
- **[Strategy spec reference](docs/STRATEGY_SPEC.md)** — the full DSL: every
  operand, condition, sizing rule, cost model and risk control. If a spec is not
  doing what you expect, this is the file that says why.
- **[Cookbook](docs/COOKBOOK.md)** — complete, runnable strategies to start from.
- **[Microstructure guide](docs/MICROSTRUCTURE.md)** — the order-book, trade,
  derivatives and cross-section feeds, and which entry point can supply them.
- **[`examples/`](examples/)** — the same strategy, runnable, in each language.
- **[Architecture](ARCHITECTURE.md)** — how the engine works, what it
  deliberately does not do, and where things live.

If the documentation is wrong or missing something, that is a bug in it: open an
issue with the documentation template.

## Questions and help

- Open a [GitHub Discussion](https://github.com/wickra-lib/wickra-backtest/discussions)
  for questions and ideas.
- Or ask with the question issue template.
- Browse [existing issues](https://github.com/wickra-lib/wickra-backtest/issues)
  first — the answer may already be there.

## Bugs and feature requests

Open a [GitHub issue](https://github.com/wickra-lib/wickra-backtest/issues) using
the bug-report or feature-request template; each has a longer variant if the
change is substantial.

For a bug, the reproducer that helps most is the one this engine runs on: the
**strategy spec**, the **bars** that trigger it, the binding and version you
used, and what you expected instead. With those three a maintainer reproduces it
in one command; without them, most of the exchange is spent reconstructing them.

## Security

Do **not** open a public issue for security problems. Report privately to
**support@wickra.org** or via GitHub private vulnerability reporting — see
[SECURITY.md](SECURITY.md).

## Support expectations

This project has a single maintainer and is supported on a best-effort basis.
Issues are triaged and acknowledged as time allows; there is no commercial
support and no service-level agreement. A clear, reproducible report gets help
fastest, and a pull request that fixes what it reports gets it fastest of all.

## Note

Backtest results are not indicative of future performance. This is a research and
engineering tool, not financial advice.
