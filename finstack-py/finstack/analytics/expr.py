"""Polars expression plugins for finstack analytics (canonical path).

Provides the same functions as ``finstack.core.analytics.expr`` but is
importable without triggering the deprecation warning for the
``finstack.core.analytics`` compatibility shim.
"""

from __future__ import annotations

import importlib.util as _util
import sys as _sys
from pathlib import Path as _Path

# Load the implementation file directly so we bypass the deprecated
# finstack.core.analytics package __init__ (which fires DeprecationWarning).
_impl_path = _Path(__file__).parent.parent / "core" / "analytics" / "expr.py"
_spec = _util.spec_from_file_location(__name__, _impl_path)
assert _spec is not None and _spec.loader is not None
_mod = _util.module_from_spec(_spec)
_sys.modules[__name__] = _mod  # pre-register before exec to prevent circular refs
_spec.loader.exec_module(_mod)  # type: ignore[union-attr]
