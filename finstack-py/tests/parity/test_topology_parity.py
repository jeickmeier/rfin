"""Topology parity tests — validates Python package structure against parity_contract.toml.

These tests verify structural placement, not behavioral correctness:

  - test_all_crates_have_python_packages: each crate in the contract has a Python package.
  - test_all_modules_exist: each module in the contract has a Python module.
  - test_no_unexpected_structural_gaps: audit script exits clean (no FAIL-level gaps).
  - test_alias_paths_resolvable: all alias old-paths are structurally resolvable.
  - test_no_leaked_helpers_in_all: packages with explicit __all__ contain no private names.

Items declared with status="missing" in the contract are expected to fail (xfail).
They will progressively be removed as Waves 2-4 land.
"""

from __future__ import annotations

import importlib
import sys
from pathlib import Path

import pytest

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

REPO_ROOT = Path(__file__).parent.parent.parent.parent  # rfin/
CONTRACT_PATH = REPO_ROOT / "finstack-py" / "parity_contract.toml"
FINSTACK_PY_ROOT = REPO_ROOT / "finstack-py"

try:
    import tomllib
except ImportError:
    import tomli as tomllib  # type: ignore[no-redef]


def _load_contract() -> dict:
    with CONTRACT_PATH.open("rb") as f:
        return tomllib.load(f)


def _package_exists(python_package: str) -> bool:
    parts = python_package.split(".")
    pkg_path = FINSTACK_PY_ROOT
    for part in parts:
        pkg_path = pkg_path / part
    return (pkg_path / "__init__.py").exists() or (pkg_path / "__init__.pyi").exists()


def _module_exists(python_module: str) -> bool:
    if _package_exists(python_module):
        return True
    parts = python_module.split(".")
    parent = FINSTACK_PY_ROOT
    for part in parts[:-1]:
        parent = parent / part
    leaf = parts[-1]
    return (parent / f"{leaf}.py").exists() or (parent / f"{leaf}.pyi").exists()


# ---------------------------------------------------------------------------
# Test 1: each crate has a Python package
# ---------------------------------------------------------------------------

def _root_package_cases() -> list[tuple[str, str, str]]:
    """Return (crate_key, python_package, declared_status) tuples."""
    contract = _load_contract()
    cases = []
    for crate_key, crate_cfg in contract.get("crates", {}).items():
        python_package = crate_cfg.get("python_package", "")
        status = crate_cfg.get("status", "unknown")
        cases.append((crate_key, python_package, status))
    return cases


@pytest.mark.parametrize("crate_key,python_package,declared_status", _root_package_cases())
def test_all_crates_have_python_packages(
    crate_key: str, python_package: str, declared_status: str
) -> None:
    """Each crate in the contract must have a corresponding Python package."""
    present = _package_exists(python_package)
    if declared_status == "missing":
        pytest.xfail(
            reason=f"Wave 2: {python_package} (crate: {crate_key}) is a planned new package"
        )
    assert present, (
        f"Python package '{python_package}' (crate: {crate_key}) does not exist. "
        f"Expected one of: {python_package.replace('.', '/')}/__init__.py or __init__.pyi"
    )


# ---------------------------------------------------------------------------
# Test 2: each module in the contract exists
# ---------------------------------------------------------------------------

def _module_cases() -> list[tuple[str, str, str, str]]:
    """Return (crate_key, module_key, python_module, declared_status) tuples."""
    contract = _load_contract()
    cases = []
    for crate_key, crate_cfg in contract.get("crates", {}).items():
        for mod_key, mod_cfg in crate_cfg.get("modules", {}).items():
            python_module = mod_cfg.get("python", "")
            status = mod_cfg.get("status", "unknown")
            cases.append((crate_key, mod_key, python_module, status))
    return cases


@pytest.mark.parametrize("crate_key,module_key,python_module,declared_status", _module_cases())
def test_all_modules_exist(
    crate_key: str, module_key: str, python_module: str, declared_status: str
) -> None:
    """Each module listed in the contract must exist as a Python package or module file."""
    present = _module_exists(python_module)
    if declared_status == "missing":
        pytest.xfail(
            reason=f"Wave 2/3: {python_module} (crate: {crate_key}, key: {module_key}) is a planned module"
        )
    assert present, (
        f"Python module '{python_module}' (crate: {crate_key}, key: {module_key}) does not exist."
    )


# ---------------------------------------------------------------------------
# Test 3: topology audit exits clean (no unexpected FAIL-level gaps)
# ---------------------------------------------------------------------------

def test_no_unexpected_structural_gaps() -> None:
    """The topology audit script must report zero unexpected failures."""
    # Import audit module dynamically so we don't need it on sys.path at collection time
    audit_script = REPO_ROOT / "scripts" / "audits" / "audit_topology.py"
    assert audit_script.exists(), f"audit_topology.py not found at {audit_script}"

    # Run the audit inline using the shared run_audit function
    spec = importlib.util.spec_from_file_location("audit_topology", audit_script)
    assert spec is not None and spec.loader is not None
    audit_mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(audit_mod)  # type: ignore[union-attr]

    report = audit_mod.run_audit(CONTRACT_PATH, FINSTACK_PY_ROOT, strict=False)
    unexpected = report["summary"]["unexpected_failures"]
    assert unexpected == 0, (
        f"{unexpected} unexpected structural gap(s) found. "
        f"Items in the contract with status!='missing' that are absent:\n"
        + "\n".join(
            f"  [FAIL] {e['python_package']} (crate: {e['crate']})"
            for e in report["root_packages"]
            if e["check"] == "FAIL"
        )
        + "\n".join(
            f"  [FAIL] {e['python_module']} (crate: {e['crate']}, key: {e['module_key']})"
            for e in report["modules"]
            if e["check"] == "FAIL"
        )
    )


# ---------------------------------------------------------------------------
# Test 4: alias old-paths are structurally resolvable
# ---------------------------------------------------------------------------

def _alias_cases() -> list[tuple[str, str]]:
    """Return (old_path, canonical_path) tuples."""
    contract = _load_contract()
    return list(contract.get("aliases", {}).items())


@pytest.mark.parametrize("old_path,canonical_path", _alias_cases())
def test_alias_paths_resolvable(old_path: str, canonical_path: str) -> None:
    """All alias old-paths must resolve to a package or module (structure check)."""
    # Check the module portion (strip trailing class name if present)
    parts = old_path.split(".")
    found = False
    for length in range(len(parts), 0, -1):
        candidate = ".".join(parts[:length])
        if _module_exists(candidate):
            found = True
            break
    assert found, (
        f"Alias old-path '{old_path}' does not resolve to any existing package or module. "
        f"Canonical target: '{canonical_path}'"
    )


# ---------------------------------------------------------------------------
# Test 5: packages with explicit __all__ contain no private names
# ---------------------------------------------------------------------------

_KNOWN_GLOBALS_BASED_PACKAGES = {
    "finstack",
    "finstack.core",
    "finstack.portfolio",
    "finstack.statements",
    "finstack.valuations",
    "finstack.valuations.calibration",
    "finstack.valuations.instruments",
}


@pytest.mark.parametrize("package", sorted(_KNOWN_GLOBALS_BASED_PACKAGES))
def test_no_leaked_helpers_in_all(package: str) -> None:
    """Packages must not leak private helpers (names starting with _) through __all__.

    This test is intentionally lenient: it only checks packages that currently
    use the globals()-based pattern. It will tighten in Wave 3 once __all__ is
    curated from the contract.
    """
    try:
        mod = importlib.import_module(package)
    except ImportError:
        pytest.skip(f"Cannot import {package} — bindings not built in this environment")

    all_exports = getattr(mod, "__all__", None)
    if all_exports is None:
        return  # No __all__ defined — nothing to check

    leaked = [name for name in all_exports if name.startswith("_")]
    assert not leaked, (
        f"Package '{package}' leaks private names through __all__: {leaked}"
    )


# ---------------------------------------------------------------------------
# Wave 4: Deprecation warning tests
# ---------------------------------------------------------------------------

_DEPRECATED_PATHS = [
    ("finstack.core.analytics",    "finstack.analytics"),
    ("finstack.statements.analysis",   "finstack.statements_analytics.analysis"),
    ("finstack.statements.templates",  "finstack.statements_analytics.templates"),
]


@pytest.mark.parametrize("old_path,canonical_path", _DEPRECATED_PATHS, ids=[p[0] for p in _DEPRECATED_PATHS])
def test_deprecated_path_emits_warning(old_path: str, canonical_path: str) -> None:
    """Importing from a deprecated alias path must emit exactly one DeprecationWarning."""
    import warnings

    # Evict any cached module so the import fires fresh
    for key in list(sys.modules.keys()):
        if old_path in key:
            del sys.modules[key]

    with warnings.catch_warnings(record=True) as w:
        warnings.simplefilter("always")
        try:
            importlib.import_module(old_path)
        except ImportError:
            pytest.skip(f"Cannot import {old_path} — bindings not built in this environment")

    dep_warnings = [x for x in w if issubclass(x.category, DeprecationWarning)]
    assert len(dep_warnings) == 1, (
        f"Expected exactly 1 DeprecationWarning from {old_path!r}, got {len(dep_warnings)}: "
        f"{[str(x.message) for x in dep_warnings]}"
    )
    assert canonical_path in str(dep_warnings[0].message), (
        f"Warning message should mention canonical path {canonical_path!r}: "
        f"{dep_warnings[0].message}"
    )


@pytest.mark.parametrize("canonical_path", [p[1] for p in _DEPRECATED_PATHS], ids=[p[1] for p in _DEPRECATED_PATHS])
def test_canonical_path_emits_no_warning(canonical_path: str) -> None:
    """Importing from a canonical path must NOT emit DeprecationWarning."""
    import warnings

    with warnings.catch_warnings(record=True) as w:
        warnings.simplefilter("always")
        try:
            importlib.import_module(canonical_path)
        except ImportError:
            pytest.skip(f"Cannot import {canonical_path}")

    dep_warnings = [x for x in w if issubclass(x.category, DeprecationWarning)]
    assert not dep_warnings, (
        f"Canonical path {canonical_path!r} should not emit DeprecationWarning, "
        f"got: {[str(x.message) for x in dep_warnings]}"
    )
