#!/usr/bin/env python3
"""Assert that every binding exposes the surface the C ABI declares.

Ten language reaches sit on one C ABI. Each has its own test suite and each is
written separately, so a reach that falls behind fails nowhere: the golden corpus
compares *values*, and a binding that never grew a method simply has no test to
run. That is not hypothetical here -- the WASM binding carries a streaming handle
no other language can reach, and nothing reported it.

The header is the source of truth. Every export in it is a promise the bindings
make, so this reads `wickra_backtest_<name>` out of
`bindings/c/include/wickra_backtest.h` and checks each language's public surface
for that name, spelled the way that language spells it.

Two exports are deliberately not part of the language surface:

  free_string   a memory-management detail of the ABI. Every binding frees the
                string it received; none of them exposes freeing as an API.
  run           the raw column-oriented entry point. Python, WASM, C# and Go wrap
                it, Node and Java expose only the JSON form; both are legitimate,
                so it is checked where it exists rather than demanded everywhere.

Extras run the other way: a binding method with no export behind it is reported
as a note, not a failure. That is how a language gets *ahead* of the ABI, which is
worth seeing but is not drift in the dangerous direction.

Run from the repository root:  python scripts/check_binding_surface.py
"""

from __future__ import annotations

import glob
import os
import re
import sys

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
HEADER = os.path.join(ROOT, "bindings", "c", "include", "wickra_backtest.h")

# Exports that are ABI plumbing rather than a promise to callers.
ABI_ONLY = {"free_string"}
# Exports a binding may legitimately wrap instead of re-exposing verbatim.
OPTIONAL = {"run"}

# How each language spells an export, where its public surface lives, and how that
# language DECLARES a name. Matching declarations rather than occurrences matters:
# a doc comment naming the function, or an internal call site, would otherwise let
# a renamed export pass unnoticed -- which is exactly what a weaker first version
# of this check did.
BINDINGS = {
    "python": (
        ["bindings/python/python/wickra_backtest/__init__.py"],
        lambda n: "__version__" if n == "version" else n,
        r"(?m)^(?:def @NAME@\(|@NAME@\s*[:=]|from .* import .*\b@NAME@\b(?!\s+as\b))",
    ),
    "node": (
        ["bindings/node/index.d.ts"],
        lambda n: re.sub(r"_(\w)", lambda m: m.group(1).upper(), n),
        r"export declare (?:function |const )@NAME@\b",
    ),
    "wasm": (
        ["bindings/wasm/src/lib.rs"],
        lambda n: n,
        r"pub fn @NAME@\s*\(",
    ),
    "csharp": (
        ["bindings/csharp/Wickra.Backtest/Backtester.cs"],
        lambda n: "".join(p.capitalize() for p in n.split("_")),
        r"public static [^\n]*\b@NAME@\s*[\({=]",
    ),
    "go": (
        ["bindings/go/backtest.go"],
        lambda n: "".join("JSON" if p == "json" else p.capitalize() for p in n.split("_")),
        r"(?m)^func @NAME@\s*\(",
    ),
    "java": (
        ["bindings/java/src/main/java/org/wickra/backtest/Backtester.java"],
        lambda n: re.sub(r"_(\w)", lambda m: m.group(1).upper(), n),
        r"public static [^\n]*\b@NAME@\s*\(",
    ),
    "r": (
        ["bindings/r/R/backtest.R"],
        lambda n: f"backtest_{n}",
        r"(?m)^@NAME@\s*<-\s*function",
    ),
}

EXPORT = re.compile(r"\bwickra_backtest_([a-z0-9_]+)\s*\(")


def declares(text: str, name: str, pattern: str) -> bool:
    """True when the language actually declares `name`, not merely mentions it."""
    return re.search(pattern.replace("@NAME@", re.escape(name)), text) is not None


def read(paths: list[str]) -> str:
    out = []
    for rel in paths:
        for path in sorted(glob.glob(os.path.join(ROOT, rel))):
            with open(path, encoding="utf-8") as handle:
                out.append(handle.read())
    return "\n".join(out)


def main() -> int:
    if not os.path.isfile(HEADER):
        print(f"header not found: bindings/c/include/wickra_backtest.h", file=sys.stderr)
        return 1
    with open(HEADER, encoding="utf-8") as handle:
        exports = sorted(set(EXPORT.findall(handle.read())))
    if not exports:
        print("no wickra_backtest_* exports found in the header", file=sys.stderr)
        return 1

    contract = [e for e in exports if e not in ABI_ONLY]
    required = [e for e in contract if e not in OPTIONAL]
    print(f"C ABI declares {len(exports)} exports; {len(required)} are required of "
          f"every binding ({', '.join(required)}).")

    failures, notes = [], []
    for lang, (paths, spell, pattern) in BINDINGS.items():
        text = read(paths)
        if not text:
            failures.append(f"{lang}: no source found at {', '.join(paths)}")
            continue
        missing = [spell(e) for e in required if not declares(text, spell(e), pattern)]
        present = [e for e in contract if declares(text, spell(e), pattern)]
        if missing:
            failures.append(f"{lang}: missing {', '.join(missing)}")
        print(f"  {lang:<7} {len(present)}/{len(contract)} of the ABI surface"
              f"{'' if not missing else '  <-- DRIFTED'}")

    # A binding that is ahead of the ABI is worth seeing, but it is not drift in
    # the direction that breaks callers.
    wasm = read(BINDINGS["wasm"][0])
    ahead = [m for m in re.findall(r"pub fn ([a-z_0-9]+)", wasm)
             if m not in contract and m not in {"new", "free"}]
    if ahead:
        notes.append("wasm exposes methods no export backs, so no other language "
                     f"can reach them: {', '.join(sorted(set(ahead)))}")

    for note in notes:
        print(f"\nnote: {note}")
    if failures:
        print("\nbinding surfaces disagree with the C ABI:", file=sys.stderr)
        for line in failures:
            print(f"  {line}", file=sys.stderr)
        return 1
    print("\nevery binding exposes the surface the C ABI declares.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
