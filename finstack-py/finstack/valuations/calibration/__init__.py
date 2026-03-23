"""Calibration bindings re-exported from Rust."""

from __future__ import annotations

from typing import Any, cast

from finstack import finstack as _finstack

_rust_calibration = cast(Any, _finstack).valuations.calibration

for _name in dir(_rust_calibration):
    if _name.startswith("_"):
        continue
    globals()[_name] = getattr(_rust_calibration, _name)

_HELPER_NAMES = frozenset({"Any", "cast", "annotations"})
__all__ = [  # pyright: ignore[reportUnsupportedDunderAll]
    name for name in globals() if not name.startswith("_") and name not in _HELPER_NAMES
]
del _HELPER_NAMES
