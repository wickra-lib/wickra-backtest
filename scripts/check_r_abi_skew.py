#!/usr/bin/env python3
"""Assert that the R binding can link against the C ABI its version names.

Every other binding ships its native code in the same artifact as its wrapper,
so the two can never disagree. R is the exception: `bindings/r/configure`
downloads a prebuilt `wickra-backtest-c-<triple>.tar.gz` from the GitHub release
named by `DESCRIPTION: Version`, and compiles `src/wickra_backtest.c` against it.
The wrapper comes from the working tree; the ABI comes from a published release.

Our own CI never sees that pairing: the R job sets `WKBT_INC` / `WKBT_LIB` and
builds against the header and library in the tree, which match by construction.
r-universe does see it, and the sibling indicator repository learned what that
costs -- its registry went red for days over 177 symbols the wrapper called that
the last release did not ship, and one export that had gained a parameter. The
skew was not a defect there and would not be one here: the wrapper is correct
against the ABI in the tree, and a release republishes both together. It is
simply invisible until a registry reports it. This makes it visible in the pull
request that opens it.

Two claims, only one of them blocking:

  * Every `wickra_backtest_*` symbol the wrapper calls must exist, with the same
    signature, in the header in this tree. A violation means the wrapper is
    stale, which is a defect and fails.
  * The same, against the header at the tag `DESCRIPTION: Version` names. A
    violation means main is ahead of the last release. The registry tracks
    `*release`, so r-universe goes on serving the previous one rather than
    failing on this one -- which makes this a release-readiness signal, not a
    defect, so it warns.

Nothing has been released yet, so the second check finds no tag and says so. It
starts doing work the moment the first one exists.

Run from the repository root:  python scripts/check_r_abi_skew.py
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
import time
import urllib.error
import urllib.request

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
HEADER = os.path.join(ROOT, "bindings", "c", "include", "wickra_backtest.h")
WRAPPER = os.path.join(ROOT, "bindings", "r", "src", "wickra_backtest.c")
DESCRIPTION = os.path.join(ROOT, "bindings", "r", "DESCRIPTION")
RAW = (
    "https://raw.githubusercontent.com/wickra-lib/wickra-backtest/{tag}"
    "/bindings/c/include/wickra_backtest.h"
)

# The wrapper defines its own `wkbt_*` helpers; only `wickra_backtest_*` crosses
# the ABI boundary.
SYMBOL = re.compile(r"\bwickra_backtest_[a-z0-9_]+\b")
COMMENT = re.compile(r"//[^\n]*|/\*.*?\*/", re.DOTALL)
TOKEN = re.compile(r"[A-Za-z_][A-Za-z0-9_]*|\*")


def released_version() -> str:
    with open(DESCRIPTION, encoding="utf-8") as handle:
        for line in handle:
            if line.startswith("Version:"):
                return line.split(":", 1)[1].strip()
    raise SystemExit(f"no Version: field in {DESCRIPTION}")


def released_header(tag: str) -> str | None:
    """The header as of `tag`, from the local clone if it has it, else raw.

    None when no release carries that tag yet, which is both what this repository
    looks like today and what a release branch looks like: DESCRIPTION already
    names the version the tag will publish.
    """
    try:
        return subprocess.run(
            ["git", "show", f"{tag}:bindings/c/include/wickra_backtest.h"],
            cwd=ROOT, check=True, capture_output=True, text=True,
        ).stdout
    except (subprocess.CalledProcessError, FileNotFoundError):
        pass
    # A CI checkout is shallow and carries no tags, so read the file from the tag
    # over the network instead. Retry: a DNS or CDN blip here would fail a job
    # that has nothing to do with the network.
    url = RAW.format(tag=tag)
    reason: Exception | None = None
    for attempt in range(1, 4):
        try:
            with urllib.request.urlopen(url, timeout=60) as response:
                return response.read().decode("utf-8")
        except urllib.error.HTTPError as err:
            # A 404 is an answer, not a flake: that tag does not exist.
            if err.code == 404:
                return None
            reason = err
            if attempt < 3:
                print(f"  attempt {attempt}/3 failed ({err}); retrying in {attempt * 5}s")
                time.sleep(attempt * 5)
        except OSError as err:
            reason = err
            if attempt < 3:
                print(f"  attempt {attempt}/3 failed ({err}); retrying in {attempt * 5}s")
                time.sleep(attempt * 5)
    raise SystemExit(f"could not read the {tag} header from {url}: {reason}")


def normalise_param(param: str) -> str:
    """A parameter reduced to its type: `struct Foo *handle` -> `struct Foo *`.

    Renaming a parameter is not an ABI change, so the name is dropped; a
    parameter that is only a type keeps it.
    """
    tokens = TOKEN.findall(param)
    if len(tokens) > 1 and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", tokens[-1]):
        tokens = tokens[:-1]
    return " ".join(tokens)


def declarations(header: str) -> dict[str, str]:
    """Map each export to its normalised return type and parameter list."""
    text = COMMENT.sub(" ", header)
    found: dict[str, str] = {}
    for statement in text.split(";"):
        match = re.search(r"\bwickra_backtest_[a-z0-9_]+\b\s*\(", statement)
        if match is None:
            continue
        name = statement[match.start(): match.end() - 1].strip()
        depth, end = 0, None
        for index in range(match.end() - 1, len(statement)):
            if statement[index] == "(":
                depth += 1
            elif statement[index] == ")":
                depth -= 1
                if depth == 0:
                    end = index
                    break
        if end is None:
            continue
        returns = " ".join(TOKEN.findall(statement[: match.start()]))
        params = statement[match.end(): end]
        signature = (
            ", ".join(normalise_param(p) for p in params.split(","))
            if params.strip()
            else "void"
        )
        found[name] = f"{returns} ({signature})"
    return found


def compare(used: set[str], declared: dict[str, str], reference: dict[str, str]) -> list[str]:
    """Symbols the wrapper calls that `declared` cannot satisfy, versus `reference`."""
    problems = []
    for name in sorted(used):
        if name not in declared:
            problems.append(f"{name}: not declared")
        elif name in reference and declared[name] != reference[name]:
            problems.append(f"{name}: declared {declared[name]}, wrapper calls {reference[name]}")
    return problems


def report(problems: list[str], limit: int = 8) -> None:
    for line in problems[:limit]:
        print(f"    {line}")
    if len(problems) > limit:
        print(f"    ... and {len(problems) - limit} more")


def main() -> int:
    with open(WRAPPER, encoding="utf-8") as handle:
        wrapper = handle.read()
    with open(HEADER, encoding="utf-8") as handle:
        tree = declarations(handle.read())

    used = set(SYMBOL.findall(wrapper))
    print(f"R wrapper calls {len(used)} C ABI exports; the header in this tree declares {len(tree)}.")

    stale = compare(used, tree, tree)
    if stale:
        print(f"\n{len(stale)} of them are absent from the header in this tree:", file=sys.stderr)
        report(stale)
        print(
            "\nbindings/r/src/wickra_backtest.c is stale -- bring it back in step"
            " with bindings/c/include/wickra_backtest.h.",
            file=sys.stderr,
        )
        return 1
    print("Every one of them matches the header in this tree.")

    version = released_version()
    tag = f"v{version}"
    header = released_header(tag)
    if header is None:
        print(
            f"\nNo release carries {tag} yet, so there is no released ABI to"
            " compare against: the tag publishes the wrapper and the ABI together."
        )
        return 0
    released = declarations(header)
    skew = compare(used, released, tree)
    print(f"\nDESCRIPTION names version {version}, whose ABI declares {len(released)} exports.")
    if not skew:
        print(f"The wrapper links against the {tag} ABI unchanged; r-universe builds green.")
        return 0

    report(skew)
    ahead = f"the R binding calls {len(skew)} C ABI exports that {tag} does not ship in that shape"
    print(
        f"\n::warning file=bindings/r/src/wickra_backtest.c::{ahead}, so r-universe"
        " cannot build it against the released library until a release republishes"
        " the two together"
    )
    print(f"\nMain is ahead of {tag}: {ahead}.")
    print(
        "This is expected between an ABI change and the release that ships it,"
        " and clears when the next release publishes wickra-backtest-c-<triple>.tar.gz."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
