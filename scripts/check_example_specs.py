#!/usr/bin/env python3
"""Hold schema/strategy_spec.schema.json against the specs it claims to describe.

The Rust side already proves the shipped specs parse: tests/example_specs.rs
feeds every one of them to `StrategySpec::parse`. Nothing proved the *schema*
still describes the same DSL. It is the contract for anyone writing a spec by
hand or generating one from a form, and it drifts the moment the parser grows a
field -- the examples get the new field, the parser accepts it, the schema is
never touched, and an editor validating against it reports the shipped examples
as invalid.

This walks the other direction from the Rust test: every key that appears in a
shipped spec must be declared somewhere in the schema, and every key the schema
marks required must be present. That is the drift that actually happens; a full
JSON Schema validator would need a third-party package, and these scripts run on
a bare `python3` in CI on purpose.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCHEMA = ROOT / "schema" / "strategy_spec.schema.json"
EXAMPLES = ROOT / "examples"


def declared_properties(schema: dict) -> set[str]:
    """Every property name the schema declares, at any nesting depth.

    Names are collected into one flat set rather than checked positionally: the
    DSL nests conditions inside operands inside indicators, and reproducing that
    walk here would be a second parser to keep in sync -- the thing this script
    exists to avoid. A flat set still catches the real failure, which is a field
    the schema has never heard of.
    """
    names: set[str] = set()

    def walk(node: object) -> None:
        if isinstance(node, dict):
            props = node.get("properties")
            if isinstance(props, dict):
                names.update(props.keys())
            for key, value in node.items():
                if key != "properties":
                    walk(value)
                else:
                    for sub in value.values():
                        walk(sub)
        elif isinstance(node, list):
            for item in node:
                walk(item)

    walk(schema)
    return names


def used_keys(node: object, acc: set[str]) -> set[str]:
    """Every object key used anywhere in a spec document."""
    if isinstance(node, dict):
        acc.update(node.keys())
        for value in node.values():
            used_keys(value, acc)
    elif isinstance(node, list):
        for item in node:
            used_keys(item, acc)
    return acc


def free_form(spec: dict) -> set[str]:
    """Keys the author chose rather than the schema naming.

    The schema has exactly one open key space -- `indicators`, declared with an
    `additionalProperties` schema so a spec can label its indicators whatever
    reads best, then refer to those labels from the rules. Everything else is a
    closed vocabulary. If a second open space is ever added, this function is
    where it has to be admitted, which is the point of naming it rather than
    loosening the comparison.
    """
    indicators = spec.get("indicators")
    return set(indicators) if isinstance(indicators, dict) else set()


def spec_paths() -> list[Path]:
    paths = [EXAMPLES / "ema-cross.json"]
    paths.extend(sorted((EXAMPLES / "strategies").glob("*.json")))
    return paths


def main() -> int:
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    declared = declared_properties(schema)
    required = set(schema.get("required", []))

    paths = spec_paths()
    if len(paths) < 7:
        print(f"error: expected the cookbook strategy set, found {len(paths)} specs")
        return 1

    failures = 0
    for path in paths:
        rel = path.relative_to(ROOT).as_posix()
        spec = json.loads(path.read_text(encoding="utf-8"))

        missing = sorted(required - set(spec))
        if missing:
            print(f"error: {rel} omits schema-required {', '.join(missing)}")
            failures += 1

        undeclared = sorted(
            key for key in used_keys(spec, set()) if key not in declared | free_form(spec)
        )
        if undeclared:
            print(f"error: {rel} uses {', '.join(undeclared)}, absent from the schema")
            failures += 1

    if failures:
        print(f"\n{failures} spec(s) disagree with {SCHEMA.relative_to(ROOT).as_posix()}")
        return 1

    print(f"ok: {len(paths)} example specs agree with the schema")
    return 0


if __name__ == "__main__":
    sys.exit(main())
