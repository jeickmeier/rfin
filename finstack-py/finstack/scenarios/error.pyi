"""Error exports for :mod:`finstack.scenarios`."""

from __future__ import annotations

from typing import Any, TypeAlias

from finstack import FinstackError

Error = FinstackError
Result: TypeAlias = Any

__all__ = [
    "Error",
    "Result",
]
