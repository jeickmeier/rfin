"""Canonical path for statements analysis tools.

This module is a structural alias for ``finstack.statements.analysis``.
Both paths resolve to the same underlying Rust module.
"""

import sys as _sys

from finstack.statements import analysis as _analysis

# Alias: make this module identical to the Rust module so that
# ``finstack.statements_analytics.analysis is finstack.statements.analysis``
_sys.modules[__name__] = _analysis
