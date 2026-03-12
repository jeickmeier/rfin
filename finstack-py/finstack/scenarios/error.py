"""Runtime error exports for :mod:`finstack.scenarios`."""

from __future__ import annotations

from typing import Any

from finstack import FinstackError

Error = FinstackError
type Result = Any

__all__ = [
    "Error",
    "Result",
]
