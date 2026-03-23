"""Three-layer parity tests — Rust binding ↔ Python runtime ↔ stub alignment.

For each canonical module declared as status=exists in parity_contract.toml:

1. Rust layer    — the symbol is accessible via ``finstack.finstack.<path>``
2. Runtime layer — the symbol is in the module's ``__all__``
3. Stub layer    — a ``.pyi`` file (or ``__init__.pyi``) exists alongside the module

These tests catch:
- Symbols that PyO3 exposes but Python packages forget to re-export.
- Packages that exist on disk but are missing stubs (IDE/type-checker blind spot).
- ``__all__`` helper-name leakage (stdlib names that sneak into exports).

All tests parametrize over the contract, so they grow automatically as new
modules are added.
"""

from __future__ import annotations

import importlib
from pathlib import Path
import types

import pytest

# ---------------------------------------------------------------------------
# Helpers to load the parity contract
# ---------------------------------------------------------------------------
REPO_ROOT = Path(__file__).parent.parent.parent.parent
CONTRACT_PATH = REPO_ROOT / "finstack-py" / "parity_contract.toml"
FINSTACK_PY_ROOT = REPO_ROOT / "finstack-py"

try:
    import tomllib
except ImportError:
    import tomli as tomllib  # type: ignore[no-redef]


def _load_contract() -> dict:
    with CONTRACT_PATH.open("rb") as f:
        return tomllib.load(f)


def _iter_present_modules() -> list[tuple[str, str, str]]:
    """Yield (crate_key, module_key, python_module) for status=exists modules."""
    contract = _load_contract()
    result = []
    for crate_key, crate_cfg in contract.get("crates", {}).items():
        for mod_key, mod_cfg in crate_cfg.get("modules", {}).items():
            if mod_cfg.get("status") == "exists":
                result.append((crate_key, mod_key, mod_cfg.get("python", "")))
    return result


def _iter_present_packages() -> list[tuple[str, str]]:
    """Yield (crate_key, python_package) for status=exists root packages."""
    contract = _load_contract()
    result = []
    for crate_key, crate_cfg in contract.get("crates", {}).items():
        if crate_cfg.get("status") == "exists":
            result.append((crate_key, crate_cfg.get("python_package", "")))
    return result


def _module_has_stub(python_module: str) -> bool:
    """Check whether a Python module path has a corresponding .pyi stub file."""
    parts = python_module.split(".")
    base = FINSTACK_PY_ROOT
    for part in parts:
        base = base / part
    # Package stub: <module>/__init__.pyi
    if (base / "__init__.pyi").exists():
        return True
    # Flat stub: parent/<leaf>.pyi
    parent = base.parent
    leaf = parts[-1]
    return (parent / f"{leaf}.pyi").exists()


def _rust_layer_names(python_module: str) -> set[str] | None:
    """Public names in the Rust extension layer for a Python module path."""
    try:
        from finstack import finstack as _fs  # type: ignore[reportMissingModuleSource]

        parts = python_module.split(".")[1:]
        mod: object = _fs
        for part in parts:
            mod = getattr(mod, part, None)
            if mod is None:
                return None
        return {
            n for n in dir(mod) if not n.startswith("_") and not isinstance(getattr(mod, n, None), types.ModuleType)
        }
    except (AttributeError, ImportError):
        return None


# Known non-API helpers that are tolerated in __all__
# (submodule attributes registered by the hybrid module setup)
_ALLOWED_NON_API = frozenset({"annotations", "Any", "cast", "ABC", "abstractmethod"})

# Modules whose __all__ is intentionally curated to a subset of Rust symbols.
# These are excluded from the Rust-vs-Python __all__ completeness check.
_INTENTIONALLY_LIMITED_ALL = frozenset({
    "finstack.core.currency",  # __all__ contains ISO-4217 constants only; Currency/get accessible but excluded from star-import
})

# PyO3 generates Iterator proxy types (e.g. ``PositionIterator``) automatically
# for Rust iterators. These are implementation artifacts — public in the Rust
# extension but not intended as user-facing Python API.
_ITERATOR_SUFFIX_PATTERNS = ("Iterator",)

# Stdlib builtins that should never appear in __all__
_STDLIB_LEAKS = frozenset(dir(__builtins__))  # type: ignore[arg-type]


# ---------------------------------------------------------------------------
# Test 1: Every present module can be imported
# ---------------------------------------------------------------------------
@pytest.mark.parametrize(
    ("crate", "mod_key", "python_module"),
    _iter_present_modules(),
    ids=[m[2] for m in _iter_present_modules()],
)
def test_present_module_is_importable(crate: str, mod_key: str, python_module: str) -> None:
    """Modules declared status=exists in the contract must be importable at runtime."""
    del crate, mod_key
    try:
        mod = importlib.import_module(python_module)
    except ImportError as exc:
        pytest.fail(f"Cannot import {python_module}: {exc}")
    assert mod is not None


# ---------------------------------------------------------------------------
# Test 2: Every present module has a stub (.pyi)
# ---------------------------------------------------------------------------
@pytest.mark.parametrize(
    ("crate", "mod_key", "python_module"),
    _iter_present_modules(),
    ids=[m[2] for m in _iter_present_modules()],
)
def test_present_module_has_stub(crate: str, mod_key: str, python_module: str) -> None:
    """Every module with status=exists must have a corresponding .pyi stub."""
    del crate, mod_key
    assert _module_has_stub(python_module), (
        f"No .pyi stub found for {python_module}. "
        f"Expected one of:\n"
        f"  {FINSTACK_PY_ROOT / python_module.replace('.', '/')}/__init__.pyi\n"
        f"  {FINSTACK_PY_ROOT / '/'.join(python_module.split('.')[:-1])}/{python_module.rsplit('.', maxsplit=1)[-1]}.pyi"
    )


# ---------------------------------------------------------------------------
# Test 3: No stdlib builtins in __all__
# ---------------------------------------------------------------------------
@pytest.mark.parametrize(
    ("crate", "python_package"),
    _iter_present_packages(),
    ids=[p[1] for p in _iter_present_packages()],
)
def test_no_stdlib_leakage_in_all(crate: str, python_package: str) -> None:
    """__all__ must not contain bare stdlib builtin names."""
    del crate
    try:
        mod = importlib.import_module(python_package)
    except ImportError:
        pytest.skip(f"Cannot import {python_package}")

    if not hasattr(mod, "__all__"):
        pytest.skip(f"{python_package} has no __all__")

    leaked = [n for n in mod.__all__ if n in _STDLIB_LEAKS and n not in _ALLOWED_NON_API]
    assert not leaked, f"{python_package}.__all__ contains stdlib names: {leaked}"


# ---------------------------------------------------------------------------
# Test 4: Runtime __all__ vs Rust layer — no unexposed names
# ---------------------------------------------------------------------------
@pytest.mark.parametrize(
    ("crate", "mod_key", "python_module"),
    [
        (crate, key, mod)
        for crate, key, mod in _iter_present_modules()
        # Only test modules where the Rust layer is the source of truth
        # (not pure-Python alias packages like statements_analytics.*)
        if not mod.startswith("finstack.statements_analytics") and not mod.startswith("finstack.analytics")
    ],
    ids=[
        m[2]
        for m in _iter_present_modules()
        if not m[2].startswith("finstack.statements_analytics") and not m[2].startswith("finstack.analytics")
    ],
)
def test_rust_symbols_accessible_from_python(crate: str, mod_key: str, python_module: str) -> None:
    """Symbols exposed by the Rust layer should be importable from the Python module.

    This catches cases where PyO3 binds a class but the Python __all__ or
    package __init__.py fails to re-export it.
    """
    del crate, mod_key
    rust_names = _rust_layer_names(python_module)
    if rust_names is None:
        pytest.skip(f"Rust layer not accessible for {python_module}")

    try:
        py_mod = importlib.import_module(python_module)
    except ImportError:
        pytest.fail(f"Cannot import {python_module}")

    py_names = set(getattr(py_mod, "__all__", []))
    if not py_names:
        pytest.skip(f"{python_module} has no __all__")

    if python_module in _INTENTIONALLY_LIMITED_ALL:
        pytest.skip(f"{python_module} has an intentionally-limited __all__")

    # Submodule names are deliberately excluded from comparison (they are
    # modules, not symbols, and are filtered in _rust_layer_names already).
    # Also exclude PyO3 iterator proxy types (implementation artifacts).
    effective_rust = {n for n in rust_names if not any(n.endswith(pat) for pat in _ITERATOR_SUFFIX_PATTERNS)}
    missing_from_python = effective_rust - py_names - _ALLOWED_NON_API
    assert not missing_from_python, (
        f"{python_module}: {len(missing_from_python)} Rust symbol(s) not in Python __all__: "
        f"{sorted(missing_from_python)[:10]}"
    )


# ---------------------------------------------------------------------------
# Test 5: Root package stubs exist
# ---------------------------------------------------------------------------
@pytest.mark.parametrize(
    ("crate", "python_package"),
    _iter_present_packages(),
    ids=[p[1] for p in _iter_present_packages()],
)
def test_root_package_has_stub(crate: str, python_package: str) -> None:
    """Root packages (crate-level) must have an __init__.pyi stub."""
    del crate
    stub = FINSTACK_PY_ROOT / python_package.replace(".", "/") / "__init__.pyi"
    assert stub.exists(), f"No __init__.pyi found for root package {python_package} at {stub}"
