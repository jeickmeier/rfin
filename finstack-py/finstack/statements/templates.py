"""Compatibility shim — canonical path is :mod:`finstack.statements_analytics.templates`.

.. deprecated::
    Import from ``finstack.statements_analytics.templates`` instead of
    ``finstack.statements.templates``.  This module will be removed in a
    future release.
"""

from __future__ import annotations

import sys as _sys
import warnings as _warnings

_warnings.warn(
    "finstack.statements.templates is deprecated. Use finstack.statements_analytics.templates instead.",
    DeprecationWarning,
    stacklevel=2,
)

from finstack import finstack as _finstack  # noqa: E402

_rust_mod = _finstack.statements.templates  # type: ignore[unresolved-attribute]

globals().update({k: v for k, v in vars(_rust_mod).items() if not k.startswith("_")})
__all__: list[str] = [n for n in dir(_rust_mod) if not n.startswith("_")]  # pyright: ignore[reportUnsupportedDunderAll]

_canonical_mod = _sys.modules.get("finstack.statements_analytics.templates")
_sys.modules[__name__] = _canonical_mod if _canonical_mod is not None else _rust_mod
