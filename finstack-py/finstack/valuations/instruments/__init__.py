"""Valuation instrument bindings re-exported from Rust."""

from __future__ import annotations

import importlib as _importlib
import sys as _sys
import types as _types
from typing import Any, cast

from finstack import finstack as _finstack

_rust_instruments = cast(Any, _finstack).valuations.instruments

for _name in dir(_rust_instruments):
    if _name.startswith("_") or _name == "evaluate_dcf":
        continue
    _attr = getattr(_rust_instruments, _name)
    globals()[_name] = _attr
    if isinstance(_attr, _types.ModuleType):
        _sys.modules[f"{__name__}.{_name}"] = _attr

for _package_name in ("commodity", "equity", "rates"):
    _sys.modules.pop(f"{__name__}.{_package_name}", None)
    globals()[_package_name] = _importlib.import_module(f"{__name__}.{_package_name}")

__all__ = [name for name in globals() if not name.startswith("_")]  # pyright: ignore[reportUnsupportedDunderAll]
