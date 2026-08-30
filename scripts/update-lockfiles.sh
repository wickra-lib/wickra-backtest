#!/usr/bin/env bash
#
# Regenerate every committed lockfile in this repository:
#   - Rust:   Cargo.lock                     (cargo update)
#   - Node:   bindings/node/package-lock.json (npm install --package-lock-only)
#   - Python: .github/requirements/*.txt      (uv pip compile --generate-hashes)
#
# Run from anywhere; it cd's to the repository root itself:
#
#     ./scripts/update-lockfiles.sh
#
# The Python locks are hash-pinned because CI installs them with
# `--require-hashes`, which is what makes the dev tooling a pinned dependency
# rather than whatever the index served that morning. They are generated with uv
# rather than pip-tools because uv resolves a *target* Python version's full
# transitive closure, with hashes, without that interpreter being installed
# locally -- which is required for the 3.9 row: it needs the exceptiongroup,
# tomli and typing-extensions backports that later versions do not.
#
# The 3.10+ file is resolved for 3.11, the floor of its matrix rows, so the
# result installs on 3.11, 3.12 and 3.13 alike. Resolving for the newest would
# risk picking something the older rows cannot install.
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v uv >/dev/null 2>&1; then
  echo "uv is not on PATH; install it from https://docs.astral.sh/uv/" >&2
  exit 1
fi

echo "== Rust =="
cargo update

echo "== Node =="
(cd bindings/node && npm install --package-lock-only --no-audit --no-fund)

echo "== Python =="
uv pip compile --generate-hashes --python-version 3.11 \
  -o .github/requirements/ci-dev-py3.txt .github/requirements/ci-dev-py3.in
uv pip compile --generate-hashes --python-version 3.9 \
  -o .github/requirements/ci-dev-py39.txt .github/requirements/ci-dev-py39.in

# The cross-library benchmark's peer libraries. bench.yml runs on 3.11 only,
# so one output suffices. The plotly pin in bench.in is load-bearing: without
# it the resolve produces a vectorbt that cannot be imported.
#
# --python-platform linux is not optional. Without it uv resolves for whatever
# machine runs this script, and on Windows the pexpect branch of ipython
# (sys_platform != "win32") is never taken -- so pexpect is absent from the
# lock. On the ubuntu runner pip then needs it, finds it unpinned, and
# --require-hashes refuses the entire install. That is what kept bench.yml red
# on three consecutive nights, unnoticed, because it only runs on a schedule.
# bench.yml runs on ubuntu and nothing else, so the lock is built for ubuntu
# and nothing else -- which also keeps the Windows-only colorama and tzdata
# out of a file that only ever installs on Linux.
uv pip compile --generate-hashes --python-version 3.11 --python-platform linux \
  -o .github/requirements/bench.txt .github/requirements/bench.in

echo
echo "Done. Review the diff, then run the version audit:"
echo "    python3 scripts/check_version_sync.py"
