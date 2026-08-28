"""The type stubs must describe the package that is actually installed.

`__init__.pyi` is hand-written -- there are four exports, so generating it the
way the main wickra repository generates its 520 would be more machinery than
surface. What generation buys there is the guarantee that the stub cannot drift,
and that is what these tests buy here instead.

Drift matters in both directions and in more than one dimension. A name that
disappears from the stub costs callers their completions; a name only the stub
has is a promise the package does not keep. Neither shows up as a failing test
in this repository -- it shows up as a type-checker error in someone else's.

The stub describes the PACKAGE, not the extension module: `run`, `run_json` and
`StreamingBacktest` are Python wrappers in `__init__.py` around the PyO3 ones,
so the package is the surface a caller sees and the thing to compare against.
"""

from __future__ import annotations

import ast
import inspect
from pathlib import Path

import wickra_backtest as wbt

STUB = Path(wbt.__file__).with_name("__init__.pyi")
# Resolved from the imported package, so in CI this is the stub inside the wheel
# that was just built and installed -- the one that actually ships, not the one
# in the source tree. If packaging ever drops it, that is this line failing.
assert STUB.is_file(), f"the installed package ships no type stub at {STUB.name}"
STUB_TREE = ast.parse(STUB.read_text(encoding="utf-8"), filename=str(STUB))

STUB_FUNCTIONS = {n.name: n for n in STUB_TREE.body if isinstance(n, ast.FunctionDef)}
STUB_CLASSES = {n.name: n for n in STUB_TREE.body if isinstance(n, ast.ClassDef)}
# Module-level annotated names, of which `__version__` is the only one.
STUB_VARIABLES = {
    n.target.id
    for n in STUB_TREE.body
    if isinstance(n, ast.AnnAssign) and isinstance(n.target, ast.Name)
}

EXPORTED = set(wbt.__all__)


def class_members(node: ast.ClassDef) -> dict[str, ast.FunctionDef]:
    """The stub's declarations for one class, dunders excluded.

    `__init__` is checked through the class's own signature and `__enter__` /
    `__exit__` through the context-manager test in test_completeness.py, so what
    is left here is the surface a caller reaches by name.
    """
    return {
        n.name: n
        for n in node.body
        if isinstance(n, ast.FunctionDef) and not n.name.startswith("_")
    }


def runtime_members(obj: type) -> set[str]:
    return {name for name in vars(obj) if not name.startswith("_")}


def parameters(node: ast.FunctionDef, *, drop_self: bool) -> list[tuple[str, bool, bool]]:
    """(name, is_keyword_only, has_default) for each parameter the stub declares.

    Types are deliberately not compared: `Sequence[float]` versus `list[float]`
    is a judgement call, whereas a parameter that has been renamed, reordered or
    lost its default is unambiguously wrong.
    """
    args = node.args
    positional = args.posonlyargs + args.args
    if drop_self:
        positional = positional[1:]
    defaults = args.defaults
    first_defaulted = len(positional) - len(defaults)
    out = [
        (arg.arg, False, index >= first_defaulted)
        for index, arg in enumerate(positional)
    ]
    out += [
        (arg.arg, True, default is not None)
        for arg, default in zip(args.kwonlyargs, args.kw_defaults)
    ]
    return out


def runtime_parameters(func) -> list[tuple[str, bool, bool]]:
    out = []
    for name, param in inspect.signature(func).parameters.items():
        if name == "self":
            continue
        out.append(
            (
                name,
                param.kind is inspect.Parameter.KEYWORD_ONLY,
                param.default is not inspect.Parameter.empty,
            )
        )
    return out


def test_every_exported_name_is_declared_in_the_stub():
    declared = set(STUB_FUNCTIONS) | set(STUB_CLASSES) | STUB_VARIABLES
    missing = sorted(EXPORTED - declared)
    assert missing == [], f"exported but not in __init__.pyi: {missing}"


def test_the_stub_declares_nothing_the_package_does_not_export():
    declared = set(STUB_FUNCTIONS) | set(STUB_CLASSES) | STUB_VARIABLES
    extra = sorted(declared - EXPORTED)
    assert extra == [], f"declared in __init__.pyi but not exported: {extra}"


def test_stub_class_members_match_the_runtime_class():
    for name, node in STUB_CLASSES.items():
        runtime = getattr(wbt, name)
        declared = set(class_members(node))
        actual = runtime_members(runtime)
        assert declared == actual, (
            f"{name}: stub declares {sorted(declared - actual)} that do not exist, "
            f"and is missing {sorted(actual - declared)}"
        )


def test_stub_properties_are_properties_at_runtime():
    for name, node in STUB_CLASSES.items():
        runtime = getattr(wbt, name)
        for member, member_node in class_members(node).items():
            decorated = any(
                isinstance(d, ast.Name) and d.id == "property"
                for d in member_node.decorator_list
            )
            is_property = isinstance(
                inspect.getattr_static(runtime, member, None), property
            )
            assert decorated == is_property, (
                f"{name}.{member}: stub says property={decorated}, "
                f"runtime says property={is_property}"
            )


def test_stub_signatures_match_the_runtime():
    """A renamed or reordered parameter is the drift a name-only check misses.

    `run(..., *, spec, capital)` is keyword-only for a reason; a stub that lost
    the star would tell every caller's type checker that a positional call is
    fine, and it would only fail at run time.
    """
    mismatches = []

    for name, node in STUB_FUNCTIONS.items():
        declared = parameters(node, drop_self=False)
        actual = runtime_parameters(getattr(wbt, name))
        if declared != actual:
            mismatches.append(f"{name}: stub {declared} != runtime {actual}")

    for class_name, node in STUB_CLASSES.items():
        runtime = getattr(wbt, class_name)
        for member, member_node in class_members(node).items():
            attribute = inspect.getattr_static(runtime, member, None)
            if isinstance(attribute, property):
                continue  # no call signature to compare
            declared = parameters(member_node, drop_self=True)
            actual = runtime_parameters(getattr(runtime, member))
            if declared != actual:
                mismatches.append(
                    f"{class_name}.{member}: stub {declared} != runtime {actual}"
                )

    init = STUB_CLASSES["StreamingBacktest"]
    stub_init = next(
        n
        for n in init.body
        if isinstance(n, ast.FunctionDef) and n.name == "__init__"
    )
    declared = parameters(stub_init, drop_self=True)
    actual = runtime_parameters(wbt.StreamingBacktest.__init__)
    if declared != actual:
        mismatches.append(f"StreamingBacktest.__init__: stub {declared} != runtime {actual}")

    assert mismatches == [], "\n".join(mismatches)
