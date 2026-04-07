"""Canonical Python path for statements-analytics analysis tools.

Maps to the ``finstack_statements_analytics::analysis`` Rust module.
"""

from __future__ import annotations

import sys as _sys

from finstack import finstack as _finstack  # type: ignore[reportAttributeAccessIssue]

# Resolve directly from the Rust extension.
_rust_analysis = _finstack.statements.analysis  # type: ignore[unresolved-attribute]
_sys.modules[__name__] = _rust_analysis
