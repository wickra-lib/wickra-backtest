#!/usr/bin/env python3
"""Binding READMEs must not use repository-relative links.

Each `bindings/*/README.md` is, or is one workflow line away from being, the long
description of a published package: PyPI renders the Python one, NuGet the C#
one, pkg.go.dev the Go one, r-universe the R one. A link like
`../../docs/COOKBOOK.md` resolves on GitHub and nowhere else -- on a registry page
it is simply broken, and nothing in the build says so, because the file it points
at does exist in the repository.

So the rule is: anything that ships as package metadata links absolutely. The
repository's own README is exempt and deliberately keeps relative links -- it is
read on GitHub far more than anywhere else, and that is the convention the main
wickra repository uses too.

Run from the repository root:  python scripts/check_readme_links.py
"""

from __future__ import annotations

import glob
import os
import re
import sys

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))

# A markdown link target that is neither absolute nor a same-page anchor. Also
# catches HTML `src=`/`href=` attributes, which the banner markup uses.
LINK = re.compile(r"\]\(\s*(?!https?://|#|mailto:)([^)\s]+)")
ATTR = re.compile(r"(?:src|href)=\"(?!https?://|#|mailto:)([^\"]+)\"")


def relative_targets(text: str) -> list[str]:
    return [m.group(1) for m in LINK.finditer(text)] + [m.group(1) for m in ATTR.finditer(text)]


def main() -> int:
    paths = sorted(glob.glob(os.path.join(ROOT, "bindings", "*", "README.md")))
    if not paths:
        print("no binding READMEs found", file=sys.stderr)
        return 1

    failures = []
    for path in paths:
        rel = os.path.relpath(path, ROOT).replace(os.sep, "/")
        with open(path, encoding="utf-8") as handle:
            found = relative_targets(handle.read())
        if found:
            failures.append(f"{rel}: {', '.join(sorted(set(found)))}")
        print(f"  {rel:<28} {'relative links: ' + str(len(found)) if found else 'all links absolute'}")

    if failures:
        print(
            "\nthese READMEs ship as package long descriptions, where a relative "
            "link is dead:",
            file=sys.stderr,
        )
        for failure in failures:
            print(f"  {failure}", file=sys.stderr)
        print(
            "\nuse https://github.com/wickra-lib/wickra-backtest/blob/main/<path> "
            "instead.",
            file=sys.stderr,
        )
        return 1

    print(f"\nall {len(paths)} binding READMEs link absolutely.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
