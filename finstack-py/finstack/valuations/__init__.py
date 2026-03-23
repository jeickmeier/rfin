"""Valuations bindings (Rust).

This package is a thin re-export of the Rust extension module.
No runtime monkeypatching or compatibility shims are applied.
"""

from __future__ import annotations

import importlib as _importlib
import sys as _sys
import types as _types

from finstack import finstack as _finstack

_rust_valuations = _finstack.valuations

for _name in dir(_rust_valuations):
    if _name.startswith("_"):
        continue
    _attr = getattr(_rust_valuations, _name)
    globals()[_name] = _attr
    if isinstance(_attr, _types.ModuleType):
        _sys.modules[f"{__name__}.{_name}"] = _attr

_sys.modules.pop(f"{__name__}.instruments", None)
instruments = _importlib.import_module(f"{__name__}.instruments")
globals()["instruments"] = instruments

_sys.modules.pop(f"{__name__}.calibration", None)
calibration = _importlib.import_module(f"{__name__}.calibration")
globals()["calibration"] = calibration

_HELPER_NAMES = frozenset({"annotations"})  # __future__ annotations feature flag
__all__ = [  # pyright: ignore[reportUnsupportedDunderAll]
    name for name in globals() if not name.startswith("_") and name not in _HELPER_NAMES
]
del _HELPER_NAMES
