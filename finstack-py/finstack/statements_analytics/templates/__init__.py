"""Canonical path for financial statement templates.

This module is a structural alias for ``finstack.statements.templates``.
Both paths resolve to the same underlying Rust module.
"""

import sys as _sys

from finstack.statements import templates as _templates

# Alias: make this module identical to the Rust module so that
# ``finstack.statements_analytics.templates is finstack.statements.templates``
_sys.modules[__name__] = _templates
