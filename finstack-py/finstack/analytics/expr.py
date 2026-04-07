"""Polars expression plugins for finstack analytics (canonical path).

Re-exports from the implementation at ``finstack/core/analytics/expr.py``.
Uses direct file loading to avoid triggering the ``finstack.core.analytics``
package ``__init__`` which raises :class:`ImportError`.
"""

from __future__ import annotations

import importlib.util as _util
from pathlib import Path as _Path
import sys as _sys

_impl_path = _Path(__file__).parent.parent / "core" / "analytics" / "expr.py"
_spec = _util.spec_from_file_location(__name__, _impl_path)
if _spec is None or _spec.loader is None:
    msg = f"Could not load analytics expr implementation from {_impl_path}"
    raise ImportError(msg)
_mod = _util.module_from_spec(_spec)
_sys.modules[__name__] = _mod  # pre-register before exec to prevent circular refs
_spec.loader.exec_module(_mod)  # type: ignore[union-attr]
