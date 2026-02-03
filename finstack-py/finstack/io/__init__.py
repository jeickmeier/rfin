"""Persistence layer for Finstack domain objects.

This module provides a typed repository interface for storing and retrieving
market contexts, instruments, portfolios, scenarios, statement models, and
metric registries. The default implementation uses SQLite.
"""

from __future__ import annotations

import sys as _sys
import types as _types

from finstack import finstack as _finstack

_rust_io = _finstack.io

for _name in dir(_rust_io):
    if _name.startswith("_"):
        continue
    _attr = getattr(_rust_io, _name)
    globals()[_name] = _attr
    if isinstance(_attr, _types.ModuleType):
        _sys.modules[f"{__name__}.{_name}"] = _attr

__all__ = [name for name in globals() if not name.startswith("_")]  # pyright: ignore[reportUnsupportedDunderAll]
