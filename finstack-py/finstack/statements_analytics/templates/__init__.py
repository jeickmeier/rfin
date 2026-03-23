"""Canonical Python path for statements-analytics templates.

Maps to the ``finstack_statements_analytics::templates`` Rust module.
"""

from __future__ import annotations

import sys as _sys

from finstack import finstack as _finstack  # type: ignore[reportAttributeAccessIssue]

_rust_templates = _finstack.statements.templates
_sys.modules[__name__] = _rust_templates
