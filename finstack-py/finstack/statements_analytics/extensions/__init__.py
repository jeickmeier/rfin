"""Canonical path for statements extension types.

This module is a structural alias for ``finstack.statements.extensions``.
Both paths resolve to the same underlying Rust module.
"""

import sys as _sys

from finstack.statements import extensions as _extensions

# Alias: make this module identical to the Rust module so that
# ``finstack.statements_analytics.extensions is finstack.statements.extensions``
_sys.modules[__name__] = _extensions
