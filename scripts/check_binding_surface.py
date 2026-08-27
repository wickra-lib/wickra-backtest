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
# The streaming surface reached the C ABI before its language wrappers did, so a
# binding may still be missing it. This is a migration in progress, not a licence:
# the gap is printed on every run, and once every binding carries a name the entry
# below is stale -- the check then fails and tells you to delete it, so the
# exemption cannot outlive the migration it was written for.
STREAMING = frozenset({
    "stream_new",
    "stream_step",
    "stream_step_json",
    "stream_equity_json",
    "stream_latest_equity_json",
    "stream_num_trades",
    "stream_finish_json",
    "stream_free",
})
# Which bindings have not grown the streaming surface yet. This is keyed by
# language on purpose: a global list of exempt names would let a binding that
# already has one quietly lose it again and still pass. An entry is a migration
# in progress, not a licence -- the moment a binding declares a name listed as
# pending for it, the check fails and says to delete the entry, so the list can
# only shrink.
PENDING = {
    "wasm": STREAMING,
    "csharp": STREAMING,
    "go": STREAMING,
    "java": STREAMING,
    "r": STREAMING,
}

# A streaming run is a class in most languages -- an object you construct, step
# and dispose -- not eight free functions. The ABI has to spell it flat; a
# binding should not. Declare the correspondence once, so a class API counts as
# the capability instead of reading as a hole.
STREAM_MEMBERS = {
    "stream_step": "step",
    "stream_step_json": "step_json",
    "stream_equity_json": "equity",
    "stream_latest_equity_json": "latest_equity",
    "stream_num_trades": "num_trades",
    "stream_finish_json": "finish",
    "stream_free": "close",
}
# Node reports through JSON strings, the way its `run` already does, so its
# members say so. A binding names things the way its own language reads; the
# mapping is what keeps that from looking like a missing capability.
NODE_MEMBERS = {
    **STREAM_MEMBERS,
    "stream_equity_json": "equity_json",
    "stream_latest_equity_json": "latest_equity_json",
    "stream_finish_json": "finish_json",
}

# For these languages, `stream_new` means the class exists and the rest mean it
# declares that member. Languages absent here are still checked flat.
CLASS_STREAMING = {
    "python": (
        STREAM_MEMBERS,
        lambda n: n,
        r"(?m)^    def @NAME@\b",
        r"(?m)^class StreamingBacktest\b",
    ),
    # index.d.ts is generated from the Rust source, so checking it proves the
    # class actually reached the published surface, not merely the crate.
    "node": (
        NODE_MEMBERS,
        lambda n: re.sub(r"_(\w)", lambda m: m.group(1).upper(), n),
        r"(?m)^  (?:get )?@NAME@\s*[(:]",
        r"export declare class StreamingBacktest\b",
    ),
}

# How each language spells an export, where its public surface lives, and how that
# language DECLARES a name. Matching declarations rather than occurrences matters:
# a doc comment naming the function, or an internal call site, would otherwise let
# a renamed export pass unnoticed -- which is exactly what a weaker first version
# of this check did.
BINDINGS = {
    "python": (
        ["bindings/python/python/wickra_backtest/__init__.py"],
        lambda n: "__version__" if n == "version" else n,
        # The last alternative is a name on its own line inside a parenthesised
        # `from ... import (...)`, which is still an import, not a mention.
        r"(?m)^(?:def @NAME@\(|@NAME@\s*[:=]|from .* import .*\b@NAME@\b(?!\s+as\b)| +@NAME@,)",
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


def exposes(lang: str, text: str, export: str, spell, pattern: str) -> bool:
    """True when `lang` offers `export`, in whichever shape that language uses."""
    members = CLASS_STREAMING.get(lang)
    if members is None or not export.startswith("stream_"):
        return declares(text, spell(export), pattern)
    member_names, mspell, mpattern, class_pattern = members
    if export == "stream_new":
        return re.search(class_pattern, text) is not None
    return declares(text, mspell(member_names[export]), mpattern)


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

    failures, notes, pending = [], [], {}
    for lang, (paths, spell, pattern) in BINDINGS.items():
        text = read(paths)
        if not text:
            failures.append(f"{lang}: no source found at {', '.join(paths)}")
            continue
        exempt = PENDING.get(lang, frozenset())
        missing = [spell(e) for e in required
                   if e not in exempt and not exposes(lang, text, e, spell, pattern)]
        pending[lang] = [e for e in required
                         if e in exempt and not exposes(lang, text, e, spell, pattern)]
        arrived = sorted(e for e in exempt if exposes(lang, text, e, spell, pattern))
        present = [e for e in contract if exposes(lang, text, e, spell, pattern)]
        if missing:
            failures.append(f"{lang}: missing {', '.join(missing)}")
        if arrived:
            failures.append(f"{lang}: now declares {', '.join(arrived)}"
                            " -- remove it from PENDING in this script")
        print(f"  {lang:<7} {len(present)}/{len(contract)} of the ABI surface"
              f"{'' if not missing else '  <-- DRIFTED'}"
              f"{'' if not pending[lang] else f'  ({len(pending[lang])} pending)'}")

    # A binding that is ahead of the ABI is worth seeing, but it is not drift in
    # the direction that breaks callers.
    wasm = read(BINDINGS["wasm"][0])
    ahead = [m for m in re.findall(r"pub fn ([a-z_0-9]+)", wasm)
             if m not in contract and m not in {"new", "free"}]
    if ahead:
        notes.append("wasm exposes methods no export backs, so no other language "
                     f"can reach them: {', '.join(sorted(set(ahead)))}")

    still = sorted({e for miss in pending.values() for e in miss})
    if still:
        notes.append("streaming wrappers still to be written; the C ABI already "
                     f"exports {', '.join(still)}")

    for note in notes:
        print(f"\nnote: {note}")
    if failures:
        print("\nbinding surfaces disagree with the C ABI:", file=sys.stderr)
        for line in failures:
            print(f"  {line}", file=sys.stderr)
        return 1
    print("\nevery binding exposes the surface the C ABI declares"
          f"{' (streaming wrappers pending)' if still else ''}.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
