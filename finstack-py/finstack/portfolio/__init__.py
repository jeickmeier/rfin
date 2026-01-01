"""Portfolio bindings (Rust).

This package is a thin re-export of the Rust extension module.
No runtime monkeypatching or compatibility shims are applied.
"""

from __future__ import annotations

import sys as _sys
import types as _types

from finstack import finstack as _finstack

_rust_portfolio = _finstack.portfolio

for _name in dir(_rust_portfolio):
    if _name.startswith("_"):
        continue
    _attr = getattr(_rust_portfolio, _name)
    globals()[_name] = _attr
    if isinstance(_attr, _types.ModuleType):
        _sys.modules[f"{__name__}.{_name}"] = _attr

__all__ = [name for name in globals() if not name.startswith("_")]
