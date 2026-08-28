#!/usr/bin/env python3
"""Run the pytest-free part of the suite, for the Python 3.9 CI row.

pytest 9.x requires Python 3.10, so the 3.9 row can only install pytest 8.4.2 --
which is below the fix for GHSA-6w46-j5rx-g56g and has no backport. Rather than
keep a vulnerable package pinned in a lock file to run tests that mostly do not
need it, the 3.9 row installs no pytest at all and runs this instead.

Most of the suite never needed it. The modules below are plain functions with
plain asserts, including the golden-corpus checks -- the ones that actually pin
cross-language equality and are therefore the ones worth running on the floor
interpreter. `test_smoke` and `test_streaming` are the exception: they use
`pytest.raises` to pin error paths, so they run on 3.10 and up, where the whole
suite runs under pytest as before.

    python bindings/python/tests/run_without_pytest.py
"""

from __future__ import annotations

import importlib
import sys
import traceback
from pathlib import Path

# The modules that import nothing but the standard library and the binding.
# Deliberately a list rather than a scan: a module that grows a pytest import
# should fail loudly here rather than be skipped silently.
MODULES = (
    "test_golden",
    "test_golden_json",
    "test_golden_streaming",
    "test_completeness",
    "test_stub_completeness",
)


def main() -> int:
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    passed, failed = 0, []

    for module_name in MODULES:
        module = importlib.import_module(module_name)
        if "pytest" in sys.modules and getattr(module, "pytest", None) is not None:
            failed.append(f"{module_name}: imports pytest, so it cannot run here")
            continue
        for name in sorted(vars(module)):
            if not name.startswith("test_"):
                continue
            function = getattr(module, name)
            if not callable(function):
                continue
            try:
                function()
                passed += 1
            except Exception:  # noqa: BLE001 - report every failure, not the first
                failed.append(f"{module_name}::{name}\n{traceback.format_exc()}")

    for failure in failed:
        print(f"FAILED {failure}", file=sys.stderr)
    print(f"{passed} passed, {len(failed)} failed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
