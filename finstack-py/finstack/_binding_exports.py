"""Shared helpers for exposing Rust extension modules through Python packages."""

from __future__ import annotations

from collections.abc import Iterable, MutableMapping
import importlib
from pathlib import Path
import sys
import types
from typing import Any


def export_rust_members(
    namespace: MutableMapping[str, Any],
    rust_module: Any,
    *,
    package_name: str,
    excluded: Iterable[str] = (),
) -> None:
    """Populate a package namespace from a Rust extension module."""
    excluded_names = frozenset(excluded)
    for name in dir(rust_module):
        if name.startswith("_") or name in excluded_names:
            continue
        attr = getattr(rust_module, name)
        namespace[name] = attr
        if isinstance(attr, types.ModuleType):
            sys.modules[f"{package_name}.{name}"] = attr


def set_public_all(
    namespace: MutableMapping[str, Any],
    *,
    helper_names: Iterable[str] = (),
) -> list[str]:
    """Compute and assign ``__all__`` from public names in a namespace."""
    excluded_names = frozenset(helper_names)
    exported_names = [name for name in namespace if not name.startswith("_") and name not in excluded_names]
    namespace["__all__"] = exported_names
    return exported_names


def setup_hybrid_module(
    rust_mod: Any,
    *,
    root_package: str,
    qualname: str,
    pkg_dir: Path,
) -> Any:
    """Prefer a Python package shim when present, otherwise register the Rust module.

    Either way, nested Rust submodules below the top level are registered in
    ``sys.modules`` so that qualified imports like ``finstack.core.dates.schedule``
    resolve straight to the compiled extension.
    """
    has_python_shim = pkg_dir.is_dir() and (pkg_dir / "__init__.py").exists()
    if has_python_shim:
        py_mod = importlib.import_module(f".{qualname}", root_package)
    else:
        sys.modules[f"{root_package}.{qualname}"] = rust_mod
        py_mod = rust_mod

    _register_nested_rust_modules(
        rust_mod,
        root_package=root_package,
        qualname=qualname,
        skip_top_level=has_python_shim,
    )
    return py_mod


def _register_nested_rust_modules(
    parent_mod: Any,
    *,
    root_package: str,
    qualname: str,
    skip_top_level: bool,
) -> None:
    """Recursively register nested Rust submodules under ``sys.modules``."""
    seen: set[int] = set()

    def recurse(mod: object, nested_qualname: str, depth: int) -> None:
        if id(mod) in seen:
            return
        seen.add(id(mod))

        for attr_name in dir(mod):
            if attr_name.startswith("_"):
                continue
            try:
                attr = getattr(mod, attr_name)
            except AttributeError:
                continue
            if isinstance(attr, types.ModuleType):
                fqname = f"{root_package}.{nested_qualname}.{attr_name}"
                if not skip_top_level or depth > 0:
                    sys.modules[fqname] = attr
                recurse(attr, f"{nested_qualname}.{attr_name}", depth + 1)

    recurse(parent_mod, qualname, 0)
