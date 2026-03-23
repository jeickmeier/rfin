"""Compatibility shim — canonical path is :mod:`finstack.statements_analytics.analysis`.

.. deprecated::
    Import from ``finstack.statements_analytics.analysis`` instead of
    ``finstack.statements.analysis``.  This module will be removed in a
    future release.
"""

from __future__ import annotations

import sys as _sys
import warnings as _warnings

_warnings.warn(
    "finstack.statements.analysis is deprecated. Use finstack.statements_analytics.analysis instead.",
    DeprecationWarning,
    stacklevel=2,
)

# Delegate to the underlying Rust module so that all symbols remain accessible
# and object identity is preserved with the canonical path.
from finstack import finstack as _finstack  # noqa: E402

_rust_mod = _finstack.statements.analysis  # type: ignore[unresolved-attribute]

# Re-export all public names and replace this module in sys.modules with the
# Rust module so that isinstance / identity checks work across both paths.
globals().update({k: v for k, v in vars(_rust_mod).items() if not k.startswith("_")})
__all__: list[str] = [n for n in dir(_rust_mod) if not n.startswith("_")]  # pyright: ignore[reportUnsupportedDunderAll]

# Register the canonical module in sys.modules under this path so subsequent
# imports skip the warning (first-import-only semantics).
_canonical_mod = _sys.modules.get("finstack.statements_analytics.analysis")
_sys.modules[__name__] = _canonical_mod if _canonical_mod is not None else _rust_mod
