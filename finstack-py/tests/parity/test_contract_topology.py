"""Structural parity checks driven by ``finstack-py/parity_contract.toml``."""

from __future__ import annotations

import importlib
import inspect
import re
from pathlib import Path
import tomllib
from typing import Any

import pytest

CONTRACT_PATH = Path(__file__).parents[2] / "parity_contract.toml"
VALID_MODULE_STATUSES = {"exists", "flattened", "missing"}


def _load_contract() -> dict[str, Any]:
    return tomllib.loads(CONTRACT_PATH.read_text())


CONTRACT = _load_contract()


def _module_entries(*statuses: str) -> list[tuple[str, str, str, str]]:
    entries: list[tuple[str, str, str, str]] = []
    for crate_name, crate in CONTRACT["crates"].items():
        for module_name, spec in crate.get("modules", {}).items():
            status = spec["status"]
            if status in statuses:
                entries.append((crate_name, module_name, spec["python"], status))
    return entries


ROOT_PACKAGES = [
    (crate_name, crate["python_package"])
    for crate_name, crate in CONTRACT["crates"].items()
    if crate.get("status") == "exists"
]

PUBLIC_MODULES = _module_entries("exists", "flattened")
MISSING_MODULES = _module_entries("missing")


def test_contract_lives_with_python_bindings() -> None:
    """The Python parity contract should be stored in the Python package tree."""
    assert Path("finstack-py/parity_contract.toml").resolve() == CONTRACT_PATH


def test_contract_uses_known_module_statuses() -> None:
    """Module status values should stay explicit and auditable."""
    unknown = [
        (crate_name, module_name, spec["status"])
        for crate_name, crate in CONTRACT["crates"].items()
        for module_name, spec in crate.get("modules", {}).items()
        if spec["status"] not in VALID_MODULE_STATUSES
    ]
    assert unknown == []


@pytest.mark.parametrize(("crate_name", "package_name"), ROOT_PACKAGES)
def test_contract_root_packages_are_importable(crate_name: str, package_name: str) -> None:
    """Every crate marked present in the contract should have an importable package."""
    assert crate_name
    importlib.import_module(package_name)


@pytest.mark.parametrize(
    ("crate_name", "module_name", "module_path", "status"),
    PUBLIC_MODULES,
)
def test_contract_public_modules_are_importable(
    crate_name: str,
    module_name: str,
    module_path: str,
    status: str,
) -> None:
    """``exists`` and ``flattened`` contract entries should resolve in Python."""
    assert crate_name
    assert module_name
    assert status in {"exists", "flattened"}
    importlib.import_module(module_path)


@pytest.mark.parametrize(
    ("crate_name", "module_name", "module_path", "status"),
    MISSING_MODULES,
)
def test_contract_missing_modules_are_not_importable(
    crate_name: str,
    module_name: str,
    module_path: str,
    status: str,
) -> None:
    """``missing`` contract entries should stay absent until the contract changes."""
    assert crate_name
    assert module_name
    assert status == "missing"
    with pytest.raises(ModuleNotFoundError) as exc_info:
        importlib.import_module(module_path)

    missing_name = exc_info.value.name
    assert missing_name is not None
    assert module_path == missing_name or module_path.startswith(f"{missing_name}.")


def _pyi_top_level_names(pyi_path: Path) -> set[str]:
    """Extract module-level public names declared in a .pyi stub.

    The regex matches lines starting with a lowercase letter, which by
    convention excludes dunders like ``__all__`` and any underscore-prefixed
    private names without needing a separate filter.
    """
    source = pyi_path.read_text()
    return {
        m.group(1)
        for m in re.finditer(r"^([a-z][a-zA-Z0-9_]*)\s*:\s*\w", source, re.MULTILINE)
    }


def test_pyi_top_level_matches_contract() -> None:
    """The `.pyi` stub, ``finstack.__all__``, and the contract must agree.

    Drift in any of the three is a maintenance hazard, since they all encode
    the same fact (the public top-level subpackages of finstack).
    """
    block = CONTRACT["pyi_top_level"]
    pyi_path = CONTRACT_PATH.parent / block["file"]
    contract = set(block["names"])
    pyi = _pyi_top_level_names(pyi_path)
    finstack_all = set(importlib.import_module("finstack").__all__)

    assert pyi == contract, (
        f"finstack.pyi top-level names diverged from contract.\n"
        f"  missing from .pyi: {sorted(contract - pyi)}\n"
        f"  unlisted in contract: {sorted(pyi - contract)}"
    )
    assert finstack_all == contract, (
        f"finstack.__all__ diverged from contract.\n"
        f"  missing from finstack.__all__: {sorted(contract - finstack_all)}\n"
        f"  unlisted in contract: {sorted(finstack_all - contract)}"
    )


def _symbol_entries() -> list[tuple[str, str, str]]:
    """Yield (crate_name, package_path, symbol_name) for every contract symbol."""
    entries: list[tuple[str, str, str]] = []
    for crate_name, crate in CONTRACT["crates"].items():
        symbols = crate.get("symbols", {})
        for sym in symbols.get("public", []):
            entries.append((crate_name, crate["python_package"], sym))
    return entries


SYMBOL_ENTRIES = _symbol_entries()


@pytest.mark.parametrize(
    ("crate_name", "package_path", "symbol_name"),
    SYMBOL_ENTRIES,
)
def test_contract_symbols_are_importable(
    crate_name: str,
    package_path: str,
    symbol_name: str,
) -> None:
    """Every contract symbol must resolve as an attribute of its package."""
    assert crate_name
    module = importlib.import_module(package_path)
    assert hasattr(module, symbol_name), (
        f"{package_path} does not expose `{symbol_name}` "
        f"(listed in parity contract under `{crate_name}.symbols.public`)"
    )


def test_contract_symbols_match_live_surface() -> None:
    """The `valuations.symbols.public` list must match the live public surface.

    Catches both directions: a public name added without contract update, and
    a contract entry that no longer exists in Python.
    """
    crate = CONTRACT["crates"]["valuations"]
    if "symbols" not in crate:
        pytest.skip("valuations has no symbols block")
    expected = set(crate["symbols"]["public"])
    module = importlib.import_module(crate["python_package"])
    submodule_names = {
        m for m, spec in crate.get("modules", {}).items()
        if spec["status"] in {"exists", "flattened"}
    }
    actual = {
        n for n in dir(module)
        if not n.startswith("_")
        and n not in submodule_names
        and not inspect.ismodule(getattr(module, n))
        and type(getattr(module, n)).__name__ != "_Feature"
    }
    assert actual == expected, (
        f"finstack.valuations public surface diverged from contract.\n"
        f"  missing from Python: {sorted(expected - actual)}\n"
        f"  unlisted in contract: {sorted(actual - expected)}"
    )
