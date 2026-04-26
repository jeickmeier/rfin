"""Structural parity checks driven by ``finstack-py/parity_contract.toml``."""

from __future__ import annotations

import importlib
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
