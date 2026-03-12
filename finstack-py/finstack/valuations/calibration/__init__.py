"""Calibration bindings re-exported from Rust."""

from __future__ import annotations

from typing import Any, cast

from finstack import finstack as _finstack

_rust_calibration = cast(Any, _finstack).valuations.calibration

for _name in dir(_rust_calibration):
    if _name.startswith("_"):
        continue
    globals()[_name] = getattr(_rust_calibration, _name)

__all__ = [name for name in globals() if not name.startswith("_")]  # pyright: ignore[reportUnsupportedDunderAll]
