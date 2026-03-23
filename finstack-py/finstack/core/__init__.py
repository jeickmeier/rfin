"""Core bindings (Rust).

This package is a thin re-export of the Rust extension module.
Submodules that have a matching Python package (e.g. analytics) are
imported via Python's normal machinery so the package ``__init__.py``
can augment the Rust module with pure-Python code.
"""

from __future__ import annotations

import importlib as _importlib
from pathlib import Path as _Path
import sys as _sys
import types as _types

from finstack import finstack as _finstack

_rust_core = _finstack.core
_pkg_dir = _Path(__file__).parent

_HAS_PYTHON_PACKAGE: set[str] = set()

for _name in dir(_rust_core):
    if _name.startswith("_"):
        continue
    _attr = getattr(_rust_core, _name)
    if isinstance(_attr, _types.ModuleType) and (_pkg_dir / _name / "__init__.py").exists():
        _py_mod = _importlib.import_module(f".{_name}", __name__)
        globals()[_name] = _py_mod
        _HAS_PYTHON_PACKAGE.add(_name)
    else:
        globals()[_name] = _attr
        if isinstance(_attr, _types.ModuleType):
            _sys.modules[f"{__name__}.{_name}"] = _attr

_HELPER_NAMES = frozenset({"annotations"})  # __future__ annotations feature flag
__all__ = [  # pyright: ignore[reportUnsupportedDunderAll]
    name for name in globals() if not name.startswith("_") and name not in _HELPER_NAMES
]
del _HELPER_NAMES
